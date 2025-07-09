import { createSignal } from 'solid-js';
import YAML from 'js-yaml';
import { $editorState, updateTestFile, type TestFile } from '../../../stores/editorState';
import MonacoEditorSolid from './MonacoEditorSolid';
import { $layoutState } from '../../../stores/layoutState';
import { EntityType, ViewMode } from '../../../stores/enum';
import { useStore } from '@nanostores/solid';

export const YamlView = () => {

    const pruneProperties = (key: string, value: string) => {
        return key === "generated" ? undefined : value
    }

    const editorState = useStore($editorState);
    let currentFile = editorState().files[editorState().currentFile];
    let currentTestFile = currentFile.type === EntityType.Test ? editorState().testFiles[currentFile.index] : undefined;

    let [data, setData] = createSignal(currentTestFile ? YAML.dump(currentTestFile.testFile, { replacer: pruneProperties }) : "");

    $editorState.subscribe((state) => {
        let file = state.files[state.currentFile];
        let testFile = file.type === EntityType.Test ? state.testFiles[file.index] : undefined;

        // only update the data signal if the file changes
        if (file.id !== currentFile.id) {
            currentFile = file;
            currentTestFile = testFile;
            setData(testFile ? YAML.dump(testFile.testFile, { replacer: pruneProperties }) : "");
        }
    });

    $layoutState.subscribe((state, _, changedKey) => {
        if (changedKey !== "viewMode") return;

        // update the data when we go to view it
        if (state.viewMode === ViewMode.RAW) {
            setData(currentTestFile ? YAML.dump(currentTestFile.testFile, { replacer: pruneProperties }) : "");
        }
    });

    const onDataChange = (value: string) => {
        try {
            let updatedFile = JSON.parse(JSON.stringify(YAML.load(value))) as TestFile;
            if (updatedFile.request) {
                updatedFile.request!.headers = updatedFile.request!.headers?.filter((h) => h !== null);
                if ((updatedFile.request!.headers?.length ?? 0) === 0) updatedFile.request!.headers = undefined;
                updatedFile.request!.params = updatedFile.request!.params?.filter((p) => p !== null);
                if ((updatedFile.request!.params?.length ?? 0) === 0) updatedFile.request!.params = undefined;
            }
            updateTestFile(updatedFile);
        } catch {
            console.log("invalid json, skipping update");
        }
    };

    return (
        <div class="flex flex-auto p-5">
            <MonacoEditorSolid value={data()} language="yaml" onChange={onDataChange} />
        </div>
    );
}
