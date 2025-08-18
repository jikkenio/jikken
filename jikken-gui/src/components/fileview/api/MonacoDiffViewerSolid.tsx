import { createEffect, createSignal, onCleanup, onMount } from 'solid-js';
import * as monaco from 'monaco-editor/esm/vs/editor/editor.api';
import 'monaco-editor/esm/vs/basic-languages/yaml/yaml.contribution';
import "monaco-editor/esm/vs/language/json/monaco.contribution";
import 'monaco-editor/esm/vs/basic-languages/html/html.contribution';
import 'monaco-editor/esm/vs/basic-languages/xml/xml.contribution';
import { configureMonacoYaml } from 'monaco-yaml';
import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import YamlWorker from './yaml.worker.js?worker';
import HtmlWorker from 'monaco-editor/esm/vs/language/html/html.worker?worker';
import JsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker';

interface MonacoDiffViewerProps {
    values: [string?, string?],
    language: string,
    // onLeftChange?: (value: string) => void,
    // onRightChange?: (value: string) => void,
    // Add other props as needed
    [key: string]: any,
};

self.MonacoEnvironment = {
    getWorker: function (_, label) {
        switch (label) {
            case 'json':
                return new JsonWorker();
            case 'yaml':
                return new YamlWorker();
            case 'html':
            case 'xml':
                return new HtmlWorker();
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
};
monaco.editor.defineTheme("vs-dark-custom", theme);

export default function MonacoDiffViewerSolid(props: MonacoDiffViewerProps) {
    const containerRef = document.createElement("div");
    containerRef.classList.add("size-full");
    let editorInstance: monaco.editor.IStandaloneDiffEditor | undefined;
    const [localValues, setLocalValues] = createSignal(props.values);

    onMount(() => {
        editorInstance = monaco.editor.createDiffEditor(containerRef, {
            theme: "vs-dark-custom",
            automaticLayout: true,
            readOnly: true,
            renderIndicators: false,
            renderOverviewRuler: false,
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
            glyphMargin: false,
            useInlineViewWhenSpaceIsLimited: false,
        });

        editorInstance.setModel({
            original: monaco.editor.createModel(props.values[0] || "", props.language),
            modified: monaco.editor.createModel(props.values[1] || "", props.language),
        });

        editorInstance.getOriginalEditor().updateOptions({ glyphMargin: false });

        // editorInstance.getOriginalEditor().onDidChangeModelContent((_) => {
        //     if (props.onLeftChange) {
        //         props.onLeftChange(editorInstance?.getOriginalEditor().getValue() || "");
        //     }
        // });

        // editorInstance.getModifiedEditor().onDidChangeModelContent((_) => {
        //     if (props.onRightChange) {
        //         props.onRightChange(editorInstance?.getModifiedEditor().getValue() || "");
        //     }
        // });
    });

    onCleanup(() => {
        if (editorInstance) {
            editorInstance.dispose();
        }
    });

    createEffect(() => {
        if (editorInstance && (props.values[0] !== localValues()[0] || props.values[1] !== localValues()[1])) {
            editorInstance.getOriginalEditor().setValue(props.values[0] ?? "");
            editorInstance.getModifiedEditor().setValue(props.values[1] ?? "");
            setLocalValues(props.values);
        }
    });

    return (
        <>{containerRef}</>
    );
};
