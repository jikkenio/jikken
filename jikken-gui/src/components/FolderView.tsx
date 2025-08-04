import { createSignal, For, Show } from "solid-js";
import { $folderState, loadFolder, openFolderDialog, removeFolder, selectEntity, toggleFolder, type FolderEntity } from "../stores/folderState";
import { createNewTestFile, openFile } from "../stores/editorState";
import { EntityType } from "../stores/enum";
import { tippy } from './TippySolid.tsx';
import { type Content } from "tippy.js";

export const FolderView = () => {

    tippy

    const [state, setState] = createSignal($folderState.get());

    const selectFile = (index: number, file: FolderEntity) => {
        selectEntity(index);
        openFile(file.type, file.name, file.path);
    };

    const closeFolder = (event: Event, folder: FolderEntity) => {
        removeFolder(folder);
        event.stopPropagation();
    }

    $folderState.subscribe((state) => {
        setState(state);
    });

    const dropdown = () => {
        return (
            <div>
                <ul class="divide-y-1 divide-neutral-600 text-neutral-300">
                    <li class="cursor-pointer hover:text-white py-1.5" onClick={() => openFolderDialog()}>Add folder</li>
                    <li class="cursor-pointer hover:text-white py-1.5" onClick={() => createNewTestFile()}>Create test file</li>
                    <li class="text-neutral-500 py-1.5">Create config file</li>
                </ul>
            </div>
        ) as Content;
    }

    return (
        <div class="flex flex-col flex-grow h-full">
            <div class="flex-none h-8 text-neutral-300 py-1 mt-1 mb-2">
                <div class="float-right mx-1 cursor-pointer hover:text-indigo-500"
                    use:tippy={{
                        props: {
                            content: dropdown(),
                            placement: "bottom-start",
                            allowHTML: true,
                            interactive: true,
                            trigger: "click",
                            delay: 0,
                            duration: 0,
                            arrow: false,
                            onShown(instance) {
                                document.querySelector('[data-tippy-root]')?.addEventListener('click', _ => {
                                    instance.hide();
                                });
                            },
                        }
                    }}>
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="16"
                        height="16"
                        fill="currentColor"
                        class="bi bi-plus-lg"
                        viewBox="0 0 16 16"
                    >
                        <path
                            fill-rule="evenodd"
                            d="M8 2a.5.5 0 0 1 .5.5v5h5a.5.5 0 0 1 0 1h-5v5a.5.5 0 0 1-1 0v-5h-5a.5.5 0 0 1 0-1h5v-5A.5.5 0 0 1 8 2"
                        ></path>
                    </svg>
                </div>
            </div>
            <Show when={state().entities.length === 0} >
                <div class="flex-auto text-neutral-400 text-center text-sm cursor-default select-none mt-8">
                    <p>No files found.</p>
                    <p>
                        <span class="cursor-pointer text-neutral-200 hover:text-white" onClick={() => openFolderDialog()}>Add a folder</span>
                        <span> or </span>
                        <span class="cursor-pointer text-neutral-200 hover:text-white" onClick={() => createNewTestFile()}>create a new file.</span>
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
                                    <Show when={entity.type === EntityType.Directory && !entity.isHidden}>
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
                                                use:tippy={{
                                                    props: {
                                                        content: "Reload contents"
                                                    }
                                                }}
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
                                                    use:tippy={{
                                                        props: {
                                                            content: "Remove from view"
                                                        }
                                                    }}
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
                                    <Show when={entity.type !== EntityType.Directory && !entity.isHidden}>
                                        <li class="group flex flex-row items-center cursor-pointer hover:text-neutral-200"
                                            style={{ "margin-left": `calc(1.5em*${entity.indentationLevel})` }}
                                            onClick={() => selectFile(index(), entity)}>
                                            <span>
                                                <svg xmlns="http://www.w3.org/2000/svg"
                                                    width="12"
                                                    height="12"
                                                    fill="currentColor"
                                                    class="bi bi-dot"
                                                    classList={{
                                                        "text-indigo-400/80 group-hover:text-indigo-400": entity.type === EntityType.Test && state().activeIndex !== index(),
                                                        "text-indigo-400": entity.type === EntityType.Test && state().activeIndex === index(),
                                                        "text-neutral-200": entity.type === EntityType.Config && state().activeIndex === index(),
                                                        "text-neutral-400 group-hover:text-neutral-200": entity.type === EntityType.Config && state().activeIndex !== index()
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
