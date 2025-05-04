import { createEffect, createSignal, onCleanup, onMount } from 'solid-js';
import * as monaco from 'monaco-editor/esm/vs/editor/editor.api';
import 'monaco-editor/esm/vs/basic-languages/yaml/yaml.contribution';
import "monaco-editor/esm/vs/language/json/monaco.contribution";
import { configureMonacoYaml } from 'monaco-yaml';
import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import YamlWorker from './yaml.worker.js?worker';

interface MonacoEditorProps {
    value?: string,
    language: string,
    readonly?: boolean,
    onChange?: (value: string) => void,
    // Add other props as needed
    [key: string]: any,
};

self.MonacoEnvironment = {
    getWorker: function (_, label) {
        const getWorkerModule = (moduleUrl: string, label: string) => {
            console.log(moduleUrl, label);
            if (!self.MonacoEnvironment?.getWorkerUrl) {
                return new EditorWorker();
            }

            return new Worker(self.MonacoEnvironment!.getWorkerUrl!(moduleUrl, label), {
                name: label,
                type: 'module'
            });
        };

        switch (label) {
            case 'json':
                return getWorkerModule('/monaco-editor/esm/vs/language/json/json.worker?worker', label);
            case 'yaml':
                return new YamlWorker();
            default:
                return new EditorWorker();
        }
    }
};

configureMonacoYaml(monaco);

const theme = {
    base: "vs-dark" as monaco.editor.BuiltinTheme,
    inherit: true,
    colors: {
        "editor.background": "#222222",
        "editor.lineHighlightBackground": "#2d2d2d",
    },
    rules: [],
}
monaco.editor.defineTheme("vs-dark-custom", theme);

export default function MonacoEditorSolid(props: MonacoEditorProps) {
    const containerRef = document.createElement("div");
<<<<<<< HEAD
    containerRef.classList.add("w-full", "min-h-36", "flex", "flex-auto");
=======
    containerRef.classList.add("w-full", "min-h-36");
>>>>>>> 330f865 (JK-594: initial move of jikken-gui project into repo)
    let editorInstance: monaco.editor.IStandaloneCodeEditor | undefined;
    const [localValue, setLocalValue] = createSignal(props.value);

    onMount(() => {
        editorInstance = monaco.editor.create(containerRef, {
            model: monaco.editor.createModel(props.value || "", props.language),
            language: props.language,
            theme: "vs-dark-custom",
            automaticLayout: true,
            formatOnType: true,
            formatOnPaste: true,
            readOnly: props.readonly || false,
            minimap: {
                enabled: false,
            },
            overviewRulerLanes: 0,
            padding: {
                top: 10,
                bottom: 10,
            },
            renderLineHighlightOnlyWhenFocus: true,
            roundedSelection: false,
            scrollBeyondLastLine: false,
        });

        editorInstance.onDidChangeModelContent((_) => {
            if (props.onChange) {
                props.onChange(editorInstance?.getValue() || "");
            }
        });
    });

    onCleanup(() => {
        if (editorInstance) {
            editorInstance.dispose();
        }
    });

    createEffect(() => {
        if (editorInstance && props.value !== localValue()) {
            editorInstance.setValue(props.value ?? "");
            setLocalValue(props.value);
        }
    });

    return (
        <>{containerRef}</>
    );
};
