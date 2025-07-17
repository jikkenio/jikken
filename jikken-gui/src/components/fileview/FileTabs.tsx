import { createSignal, For } from "solid-js";
import { $editorState, addNewFile, closeFile, selectFile, type FileState } from "../../stores/editorState";
import { $layoutState, setViewMode } from "../../stores/layoutState";
import { EntityType, ViewMode } from "../../stores/enum";

export const FileTabs = () => {

    const [layoutState, setLayoutState] = createSignal($layoutState.get());
    const [editorState, setEditorState] = createSignal($editorState.get());

    $layoutState.subscribe((state) => {
        setLayoutState(state);
    });

    $editorState.subscribe((state) => {
        setEditorState(state);
    });

    const trySelectFile = (index: number) => {
        if (index === editorState().currentFile) return;
        selectFile(index);
    };

    const tryCloseFile = (e: Event, index: number) => {
        closeFile(index);
        e.stopPropagation();
    };

    const getTabName = (file: FileState) => {
        if (file?.file) return file.file.name;

        let testFile = file.type === EntityType.Test ? editorState().testFiles[file.index]?.testFile : undefined;
        if (testFile && testFile.request?.url) {
            let method = testFile!.request.method?.toUpperCase().concat(" ") || "";
            return `${method}${testFile!.request.url}`;
        }

        return "Scratch Pad";
    };

    return (
        <div>
            <nav class="flex divide-x divide-neutral-700 shadow select-none h-12">
                <For each={editorState().files}>
                    {(file, index) => (
                        <div
                            class="group flex flex-[2_1_auto] justify-between min-w-8 max-w-72 w-8 overflow-hidden pl-2 pr-2 text-center text-sm font-medium focus:z-10"
                            classList={{
                                "border-b border-neutral-700 cursor-pointer bg-neutral-900 text-neutral-500 hover:bg-neutral-800 hover:text-neutral-300": index() !== editorState().currentFile,
                                "cursor-default bg-neutral-850 text-neutral-300": index() === editorState().currentFile
                            }}
                            onClick={[trySelectFile, index()]}
                        >
                            <span class="flex flex-row">
                                <span class="my-auto mr-2">
                                    <svg xmlns="http://www.w3.org/2000/svg"
                                        width="16"
                                        height="16"
                                        fill="currentColor"
                                        class="bi bi-dot"
                                        classList={{
                                            "text-indigo-500": file.type === EntityType.Test && index() === editorState().currentFile,
                                            "text-indigo-500/80 group-hover:text-indigo-500": file.type === EntityType.Test && index() !== editorState().currentFile,
                                            "text-neutral-300": file.type === EntityType.Config && index() === editorState().currentFile,
                                            "test-neutral-400 group-hover:text-neutral-300": file.type === EntityType.Config && index() !== editorState().currentFile,
                                        }}
                                        viewBox="0 0 16 16">
                                        <path d="M8 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6" />
                                    </svg>
                                </span>
                                <span class="my-auto select-none truncate">{getTabName(file)}</span>
                            </span>
                            <span class="p-1 my-auto text-neutral-400 invisible cursor-pointer group-hover:visible hover:text-white"
                                onClick={(e) => tryCloseFile(e, index())}>
                                <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" fill="currentColor" class="bi bi-x" viewBox="0 0 16 16">
                                    <path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708" />
                                </svg>
                            </span>
                        </div>
                    )}
                </For>
                <div
                    class="group flex flex-none bg-neutral-900 px-4 border-b border-neutral-700 cursor-pointer text-center hover:bg-indigo-600 hover:text-white focus:z-10"
                    onClick={addNewFile}
                >
                    <span class="my-auto">
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="16"
                            height="16"
                            class="bi bi-plus-lg stroke-1 stroke-indigo-500 group-hover:stroke-white"
                            viewBox="0 0 16 16"
                        >
                            <path
                                fill-rule="evenodd"
                                d="M8 2a.5.5 0 0 1 .5.5v5h5a.5.5 0 0 1 0 1h-5v5a.5.5 0 0 1-1 0v-5h-5a.5.5 0 0 1 0-1h5v-5A.5.5 0 0 1 8 2"
                            ></path>
                        </svg>
                    </span>
                </div>
                <div class="flex flex-1 bg-neutral-900 border-b border-neutral-700 items-center">
                    <div class="flex rounded-md ml-auto h-8 px-3">
                        <button type="button" title="API Mode"
                            onClick={() => setViewMode(ViewMode.API)}
                            class="w-10 flex items-center rounded-l-md px-3 py-2 text-sm font-semibold focus:z-10"
                            classList={{
                                "bg-indigo-600 text-white cursor-default": layoutState().viewMode === ViewMode.API,
                                "bg-transparent text-neutral-400 border border-1 border-r-0 border-neutral-700 hover:bg-indigo-400 hover:text-white hover:border-indigo-400": layoutState().viewMode !== ViewMode.API,
                            }}>
                            <svg xmlns="http://www.w3.org/2000/svg"
                                width="16"
                                height="16"
                                fill="currentColor"
                                class="bi bi-send-fill"
                                viewBox="0 0 16 16">
                                <path d="M15.964.686a.5.5 0 0 0-.65-.65L.767 5.855H.766l-.452.18a.5.5 0 0 0-.082.887l.41.26.001.002 4.995 3.178 3.178 4.995.002.002.26.41a.5.5 0 0 0 .886-.083zm-1.833 1.89L6.637 10.07l-.215-.338a.5.5 0 0 0-.154-.154l-.338-.215 7.494-7.494 1.178-.471z" />
                            </svg>
                        </button>
                        <button type="button" title="YAML Mode"
                            onClick={() => setViewMode(ViewMode.RAW)}
                            class="w-10 flex items-center -ml-px rounded-r-md px-3 py-2 text-sm font-semibold focus:z-10"
                            classList={{
                                "bg-indigo-600 text-white": layoutState().viewMode === ViewMode.RAW,
                                "bg-transparent text-neutral-400 border border-1 border-l-0 border-neutral-700 hover:bg-indigo-400 hover:text-white hover:border-indigo-400": layoutState().viewMode !== ViewMode.RAW,
                            }}>
                            <svg xmlns="http://www.w3.org/2000/svg"
                                width="16"
                                height="16"
                                fill="currentColor"
                                class="bi bi-card-text"
                                viewBox="0 0 16 16">
                                <path d="M14.5 3a.5.5 0 0 1 .5.5v9a.5.5 0 0 1-.5.5h-13a.5.5 0 0 1-.5-.5v-9a.5.5 0 0 1 .5-.5zm-13-1A1.5 1.5 0 0 0 0 3.5v9A1.5 1.5 0 0 0 1.5 14h13a1.5 1.5 0 0 0 1.5-1.5v-9A1.5 1.5 0 0 0 14.5 2z" />
                                <path d="M3 5.5a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9a.5.5 0 0 1-.5-.5M3 8a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9A.5.5 0 0 1 3 8m0 2.5a.5.5 0 0 1 .5-.5h6a.5.5 0 0 1 0 1h-6a.5.5 0 0 1-.5-.5" />
                            </svg>
                        </button>
                    </div>
                </div>
            </nav>
        </div >
    );
}
