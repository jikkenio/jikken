import { useStore } from '@nanostores/solid';
import YAML from 'js-yaml';
import { $editorState, updateFile, type TestFile } from '../../stores/editorState';
import MonacoEditorSolid from './MonacoEditorSolid';
<<<<<<< HEAD
<<<<<<< HEAD
import { $layoutState, ViewMode } from '../../stores/layoutState';
=======
>>>>>>> 330f865 (JK-594: initial move of jikken-gui project into repo)
=======
>>>>>>> d5b23c1 (JK-592: upgrade frontend packages and migrate to tailwind 4. this may have broken some functionality)

export const YamlView = () => {
    const editorState = useStore($editorState);

    const pruneProperties = (key: string, value: string) => {
        return key === "generated" ? undefined : value
    }

<<<<<<< HEAD
    let editorState = $editorState.get();
<<<<<<< HEAD
    let currentFile = editorState.files[editorState.currentFile];

    let [data, setData] = createSignal(YAML.dump(editorState.files[editorState.currentFile].testFile, { replacer: pruneProperties }));

    $editorState.subscribe((state) => {
        let file = state.files[state.currentFile];
        console.log(file.testFile);

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
=======
    let [data, setData] = createSignal(YAML.dump(editorState.files[editorState.currentFile].testFile, { replacer: pruneProperties }));

    $editorState.subscribe((state) => {
        let file = state.files[state.currentFile].testFile;
        setData(YAML.dump(file, { replacer: pruneProperties }));
>>>>>>> 330f865 (JK-594: initial move of jikken-gui project into repo)
    });
=======
    const data = () => {
        const currentFile = editorState().files[editorState().currentFile];
        return YAML.dump(currentFile.testFile, { replacer: pruneProperties });
    };
>>>>>>> d5b23c1 (JK-592: upgrade frontend packages and migrate to tailwind 4. this may have broken some functionality)

    const onDataChange = (value: string) => {
        try {
<<<<<<< HEAD
=======
            // setData(value);
>>>>>>> 330f865 (JK-594: initial move of jikken-gui project into repo)
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
