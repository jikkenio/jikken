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
        console.log('Monaco Editor: onMount started - using ResizeObserver for container changes');

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

        console.log('Monaco Editor: editor instance created with automaticLayout=false');

        editorInstance.onDidChangeModelContent((_) => {
            if (props.onChange) {
                props.onChange(editorInstance?.getValue() || "");
            }
        });

        // Use ResizeObserver to watch the container directly
        const resizeObserver = new ResizeObserver(() => {
            console.log('Monaco Editor: ResizeObserver detected container size change');
            if (editorInstance) {
                // Force Monaco to recalculate its layout
                editorInstance.layout();
            }
        });

        // Start observing the container
        resizeObserver.observe(containerRef);

        // Also listen for window resize as backup
        const handleWindowResize = () => {
            console.log('Monaco Editor: handling window resize event');
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
                console.log('Monaco Editor: performing initial layout');
                editorInstance.layout();
            }
        }, 100);
    });

    onCleanup(() => {
        console.log('Monaco Editor: cleanup started');
        if (editorInstance) {
            // Remove the ResizeObserver
            const resizeObserver = (editorInstance as any)._resizeObserver;
            if (resizeObserver) {
                console.log('Monaco Editor: disconnecting ResizeObserver');
                resizeObserver.disconnect();
            }
            
            // Remove the window resize listener
            const windowResizeListener = (editorInstance as any)._windowResizeListener;
            if (windowResizeListener) {
                console.log('Monaco Editor: removing window resize listener');
                window.removeEventListener('resize', windowResizeListener);
            }
            
            console.log('Monaco Editor: disposing editor instance');
            editorInstance.dispose();
        }
    });

    createEffect(() => {
        console.log('Monaco Editor: createEffect triggered', { 
            hasEditorInstance: !!editorInstance, 
            propsValue: props.value, 
            localValue: localValue(),
            valueChanged: props.value !== localValue()
        });
        
        if (editorInstance && props.value !== localValue()) {
            console.log('Monaco Editor: setting new value');
            editorInstance.setValue(props.value ?? "");
            setLocalValue(props.value);
        }
    });

    return (
        <>{containerRef}</>
    );
};
