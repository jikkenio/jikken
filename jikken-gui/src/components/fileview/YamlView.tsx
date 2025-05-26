import { createSignal } from 'solid-js';
import YAML from 'js-yaml';
import { $editorState, updateFile, type TestFile } from '../../stores/editorState';
import MonacoEditorSolid from './MonacoEditorSolid';
import { $layoutState, ViewMode } from '../../stores/layoutState';

export const YamlView = () => {

    const pruneProperties = (key: string, value: string) => {
        return key === "generated" ? undefined : value
    }

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile];

    let [data, setData] = createSignal(YAML.dump(editorState.files[editorState.currentFile].testFile, { replacer: pruneProperties }));

    $editorState.subscribe((state) => {
        let file = state.files[state.currentFile];

        // only update the data signal if the file changes
        if (file.id !== currentFile.id) {
            currentFile = file;
            setData(YAML.dump(file.testFile, { replacer: pruneProperties }));
        }
    });

    $layoutState.subscribe((state, changedKey) => {
        if (changedKey !== "viewMode") return;

        // update the data when we go to view it
        if (state.viewMode === ViewMode.RAW) {
            setData(YAML.dump(currentFile.testFile, { replacer: pruneProperties }));
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
            updateFile(updatedFile);
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
