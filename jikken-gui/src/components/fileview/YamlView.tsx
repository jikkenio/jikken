import { createSignal } from 'solid-js';
import YAML from 'js-yaml';
import { $editorState, updateFile, type TestFile } from '../../stores/editorState';
import MonacoEditorSolid from './MonacoEditorSolid';

export const YamlView = () => {

    const pruneProperties = (key: string, value: string) => {
        return key === "generated" ? undefined : value
    }

    let editorState = $editorState.get();
    let [data, setData] = createSignal(YAML.dump(editorState.files[editorState.currentFile].testFile, { replacer: pruneProperties }));

    $editorState.subscribe((state) => {
        let file = state.files[state.currentFile].testFile;
        setData(YAML.dump(file, { replacer: pruneProperties }));
    });

    const onDataChange = (value: string) => {
        try {
            // setData(value);
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
