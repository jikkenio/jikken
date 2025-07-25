import { createSignal } from 'solid-js';
import { $editorState, type ConfigFile, type ConfigSettings, updateConfigFile } from '../../../stores/editorState';
import MonacoEditorSolid from './MonacoEditorSolid';
import { $layoutState } from '../../../stores/layoutState';
import { EntityType } from '../../../stores/enum';
import { useStore } from '@nanostores/solid';
import toml from '@iarna/toml';

export const TomlView = () => {

    type ConfigToml = {
        settings?: ConfigSettings,
        globals?: Object,
    };

    const toToml = (config: ConfigFile) => {
        let configToml: ConfigToml = { settings: config.settings };
        if (config.globals) {
            configToml.globals = Object.fromEntries(config.globals!.entries());
        }

        return toml.stringify(configToml);
    };

    const editorState = useStore($editorState);
    let currentFile = editorState().files[editorState().currentFile];
    let currentConfig = currentFile.type === EntityType.Config ? editorState().configFiles[currentFile.index] : undefined;

    let [data, setData] = createSignal(currentConfig ? toToml(currentConfig) : "");

    $editorState.subscribe((state) => {
        let file = state.files[state.currentFile];
        let config = file.type === EntityType.Config ? state.configFiles[file.index] : undefined;

        // only update the data signal if the file changes to prevent the cursor resetting
        if (file.id !== currentFile.id) {
            setData(config ? toToml(config) : "");
        }

        currentFile = file;
        currentConfig = config;
    });

    $layoutState.subscribe((_new, _old, changedKey) => {
        if (changedKey !== "viewMode") return;

        // update data signal when the view is changed
        setData(currentConfig ? toToml(currentConfig) : "");
    });

    const onDataChange = (value: string) => {
        try {
            let tomlValue: ConfigToml = JSON.parse(JSON.stringify(toml.parse(value)));
            let config: ConfigFile = { settings: tomlValue.settings };
            if (tomlValue.globals) {
                config.globals = new Map(Object.entries(tomlValue.globals!));
            }
            updateConfigFile(config);
        } catch {
            console.log("invalid json, skipping update");
        }
    };

    return (
        <div class="flex flex-auto p-5">
            <MonacoEditorSolid value={data()} language="toml"
                onChange={onDataChange}
            />
        </div>
    );
}
