import { For, Show } from "solid-js";
import { useStore } from "@nanostores/solid";
import { $folderState, loadFolder, openFolderDialog, removeFolder, selectEntity, toggleFolder, type FolderEntity } from "../stores/folderState";
import { openFile } from "../stores/editorState";

export const FolderView = () => {
    const state = useStore($folderState);

    const selectFile = (index: number, file: FolderEntity) => {
        selectEntity(index);
        openFile(file);
    };

    const closeFolder = (event: Event, folder: FolderEntity) => {
        removeFolder(folder);
        event.stopPropagation();
    }

    return (
        <div class="flex flex-col flex-grow h-full">
            <div class="flex-none h-8 text-white py-1 mb-2">
                <div class="float-right mx-1 cursor-pointer hover:text-indigo-500" onClick={() => openFolderDialog()}>
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="20"
                        height="20"
                        fill="currentColor"
                        class="bi bi-folder-plus"
                        viewBox="0 0 16 16">
                        <path d="m.5 3 .04.87a2 2 0 0 0-.342 1.311l.637 7A2 2 0 0 0 2.826 14H9v-1H2.826a1 1 0 0 1-.995-.91l-.637-7A1 1 0 0 1 2.19 4h11.62a1 1 0 0 1 .996 1.09L14.54 8h1.005l.256-2.819A2 2 0 0 0 13.81 3H9.828a2 2 0 0 1-1.414-.586l-.828-.828A2 2 0 0 0 6.172 1H2.5a2 2 0 0 0-2 2m5.672-1a1 1 0 0 1 .707.293L7.586 3H2.19q-.362.002-.683.12L1.5 2.98a1 1 0 0 1 1-.98z" />
                        <path d="M13.5 9a.5.5 0 0 1 .5.5V11h1.5a.5.5 0 1 1 0 1H14v1.5a.5.5 0 1 1-1 0V12h-1.5a.5.5 0 0 1 0-1H13V9.5a.5.5 0 0 1 .5-.5" />
                    </svg>
                </div>
            </div>
            <Show when={state().entities.length === 0} >
                <div class="flex-auto text-neutral-400 text-center text-sm cursor-default select-none mt-8">
                    <p>No files found.</p>
                    <p>
                        <span class="cursor-pointer text-neutral-200 hover:text-white" onClick={() => openFolderDialog()}>Add folders</span>
                        <span> to view them here.</span>
                    </p>
                </div>
            </Show>
            <Show when={state().entities.length > 0} >
                <div class="flex-auto text-neutral-400 text-center text-sm">
                    <ul class="text-left p-2 px-3">
                        <For each={state().entities}>
                            {(entity, index) => (
                                <div class="py-px" classList={{
                                    "bg-neutral-700/70 text-neutral-200": state().activeIndex === index()
                                }}>
                                    <Show when={entity.isDirectory && !entity.isHidden}>
                                        <li class="group flex flex-row items-center cursor-pointer hover:text-neutral-200"
                                            style={{ "margin-left": `calc(1.5em*${entity.indentationLevel})` }}
                                            onClick={() => toggleFolder(entity)}>
                                            <Show when={entity.isExpanded}>
                                                <span>
                                                    <svg xmlns="http://www.w3.org/2000/svg"
                                                        width="12"
                                                        height="12"
                                                        fill="currentColor"
                                                        class="bi bi-chevron-down text-neutral-400/80 group-hover:text-neutral-200"
                                                        viewBox="0 0 16 16">
                                                        <path fill-rule="evenodd" d="M1.646 4.646a.5.5 0 0 1 .708 0L8 10.293l5.646-5.647a.5.5 0 0 1 .708.708l-6 6a.5.5 0 0 1-.708 0l-6-6a.5.5 0 0 1 0-.708" />
                                                    </svg>
                                                </span>
                                            </Show>
                                            <Show when={!entity.isExpanded}>
                                                <span>
                                                    <svg xmlns="http://www.w3.org/2000/svg"
                                                        width="12"
                                                        height="12"
                                                        fill="currentColor"
                                                        class="bi bi-chevron-right text-neutral-400/80 group-hover:text-neutral-200"
                                                        viewBox="0 0 16 16">
                                                        <path fill-rule="evenodd" d="M4.646 1.646a.5.5 0 0 1 .708 0l6 6a.5.5 0 0 1 0 .708l-6 6a.5.5 0 0 1-.708-.708L10.293 8 4.646 2.354a.5.5 0 0 1 0-.708" />
                                                    </svg>
                                                </span>
                                            </Show>
                                            <span class="ml-1.5 truncate flex-grow">{entity.name}</span>
                                            <span
                                                class="flex-none cursor-pointer text-neutral-300"
                                                onClick={() => loadFolder(entity.path)}
                                            >
                                                <svg xmlns="http://www.w3.org/2000/svg"
                                                    width="12"
                                                    height="12"
                                                    fill="currentColor"
                                                    class="bi bi-arrow-clockwise invisible group-hover:visible hover:stroke-1 hover:stroke-indigo-500 hover:fill-indigo-500"
                                                    viewBox="0 0 16 16">
                                                    <path fill-rule="evenodd" d="M8 3a5 5 0 1 0 4.546 2.914.5.5 0 0 1 .908-.417A6 6 0 1 1 8 2z" />
                                                    <path d="M8 4.466V.534a.25.25 0 0 1 .41-.192l2.36 1.966c.12.1.12.284 0 .384L8.41 4.658A.25.25 0 0 1 8 4.466" />
                                                </svg>
                                            </span>
                                            <Show when={entity.indentationLevel === 0}>
                                                <span
                                                    class="flex-none cursor-pointer text-neutral-300 ml-1"
                                                    onClick={(e) => closeFolder(e, entity)}
                                                >
                                                    <svg xmlns="http://www.w3.org/2000/svg"
                                                        width="12"
                                                        height="12"
                                                        fill="currentColor"
                                                        class="bi bi-x-lg invisible group-hover:visible hover:stroke-1 hover:stroke-indigo-500"
                                                        viewBox="0 0 16 16">
                                                        <path d="M2.146 2.854a.5.5 0 1 1 .708-.708L8 7.293l5.146-5.147a.5.5 0 0 1 .708.708L8.707 8l5.147 5.146a.5.5 0 0 1-.708.708L8 8.707l-5.146 5.147a.5.5 0 0 1-.708-.708L7.293 8z" />
                                                    </svg>
                                                </span>
                                            </Show>
                                        </li>
                                    </Show>
                                    <Show when={!entity.isDirectory && !entity.isHidden}>
                                        <li class="group flex flex-row items-center cursor-pointer hover:text-neutral-200"
                                            style={{ "margin-left": `calc(1.5em*${entity.indentationLevel})` }}
                                            onClick={() => selectFile(index(), entity)}>
                                            <span>
                                                <svg xmlns="http://www.w3.org/2000/svg"
                                                    width="12"
                                                    height="12"
                                                    fill="currentColor"
                                                    class="bi bi-dot text-indigo-400/80 group-hover:text-indigo-400"
                                                    classList={{
                                                        "text-indigo-400": state().activeIndex === index()
                                                    }}
                                                    viewBox="0 0 16 16">
                                                    <path d="M8 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6" />
                                                </svg>
                                            </span>
                                            <span class="ml-1.5 truncate">{entity.name}</span>
                                        </li>
                                    </Show>
                                </div>
                            )}
                        </For>
                    </ul>
                </div>
            </Show>
        </div>
    );
};
