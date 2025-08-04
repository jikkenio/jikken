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
}
monaco.editor.defineTheme("vs-dark-custom", theme);

export default function MonacoEditorSolid(props: MonacoEditorProps) {
    const containerRef = document.createElement("div");
    containerRef.classList.add("w-full", "h-full");
    let editorInstance: monaco.editor.IStandaloneCodeEditor | undefined;
    const [localValue, setLocalValue] = createSignal(props.value);

    onMount(() => {
        editorInstance = monaco.editor.create(containerRef, {
            model: monaco.editor.createModel(props.value || "", props.language),
            language: props.language,
            theme: "vs-dark-custom",
            automaticLayout: false, // Disable automatic layout, we'll handle it manually
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

        // Use ResizeObserver to watch the container directly
        const resizeObserver = new ResizeObserver(() => {
            if (editorInstance) {
                // Force Monaco to recalculate its layout
                editorInstance.layout();
            }
        });

        // Start observing the container
        resizeObserver.observe(containerRef);

        // Also listen for window resize as backup
        const handleWindowResize = () => {
            if (editorInstance) {
                editorInstance.layout();
            }
        };

        window.addEventListener('resize', handleWindowResize);

        // Store both listeners for cleanup
        (editorInstance as any)._resizeObserver = resizeObserver;
        (editorInstance as any)._windowResizeListener = handleWindowResize;

        // Initial layout call to ensure proper sizing
        setTimeout(() => {
            if (editorInstance) {
                editorInstance.layout();
            }
        }, 100);
    });

    onCleanup(() => {
        if (editorInstance) {
            // Remove the ResizeObserver
            const resizeObserver = (editorInstance as any)._resizeObserver;
            if (resizeObserver) {
                resizeObserver.disconnect();
            }

            // Remove the window resize listener
            const windowResizeListener = (editorInstance as any)._windowResizeListener;
            if (windowResizeListener) {
                window.removeEventListener('resize', windowResizeListener);
            }

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
