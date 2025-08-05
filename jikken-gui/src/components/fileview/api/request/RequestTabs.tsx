import { createSignal, For, Show } from "solid-js";
import { $layoutState, setRequestTabActive } from "../../../../stores/layoutState";
import { Parameters } from "./Parameters";
import { Headers } from "./Headers";
import { Body } from './Body';
import { Auth } from './Auth';
import { $editorState, saveCurrentFile } from "../../../../stores/editorState";
import { EntityType } from "../../../../stores/enum";
import { tippy } from '../../../TippySolid.tsx';

export const RequestTabs = () => {

    tippy

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile];
    let currentTestFile = currentFile.type === EntityType.Test ? editorState.testFiles[currentFile.index] : undefined;

    const [isSaveable, setIsSaveable] = createSignal(currentTestFile?.testFile.request?.url ? true : false);
    const [layoutState, setLayoutState] = createSignal($layoutState.get());

    $editorState.subscribe((state) => {
        let file = state.files[state.currentFile];
        let testFile = file.type === EntityType.Test ? state.testFiles[file.index] : undefined;
        setIsSaveable(testFile?.testFile.request?.url ? true : false);
    });

    $layoutState.subscribe((state) => {
        setLayoutState(state);
    });

    return (
        <div class="size-full flex flex-col min-w-0">
            <div class="flex-none">
                <div class="flex items-center justify-between border-b border-neutral-800 m-3 mt-1">
                    <nav class="-mb-px flex space-x-4" aria-label="Tabs">
                        <For each={layoutState().requestTabs}>
                            {(tab) => (
                                <div
                                    class="group flex min-w-12 justify-center whitespace-nowrap border-b-2 py-2 text-sm cursor-pointer font-medium"
                                    classList={{
                                        "border-indigo-500 text-neutral-300": tab.index === layoutState().requestTabIndex,
                                        "border-transparent text-neutral-400 hover:text-neutral-200": tab.index !== layoutState().requestTabIndex
                                    }}
                                    onClick={() => setRequestTabActive(tab.index)}
                                >
                                    {tab.label}

                                    <Show when={tab.showCount && tab.items > 0}>
                                        <span
                                            class="ml-1.5 my-auto rounded-[4px] px-[5px] py-px text-xs font-medium inline-block"
                                            classList={{
                                                "bg-indigo-500 text-white": tab.index === layoutState().requestTabIndex,
                                                "bg-neutral-850 ring-1 ring-inset ring-neutral-600 text-neutral-300 group-hover:text-black group-hover:bg-neutral-300 group-hover:ring-0": tab.index !== layoutState().requestTabIndex,
                                            }}
                                        >
                                            {tab.items}
                                        </span>
                                    </Show>

                                    <Show when={!tab.showCount && tab.items > 0}>
                                        <span
                                            class="ml-1.5 my-auto inline-block"
                                            classList={{
                                                "text-indigo-500": tab.index === layoutState().requestTabIndex,
                                                "text-neutral-400 group-hover:text-neutral-300": tab.index !== layoutState().requestTabIndex,
                                            }}
                                        >
                                            <svg xmlns="http://www.w3.org/2000/svg"
                                                width="6"
                                                height="16"
                                                fill="currentColor"
                                                class="bi bi-dot"
                                                viewBox="6 0 3 16">
                                                <path d="M8 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6" />
                                            </svg>
                                        </span>
                                    </Show>
                                </div>
                            )}
                        </For>
                    </nav>
                    <Show when={isSaveable()}>
                        <span class="flex-none text-sm p-2 mr-1 cursor-pointer text-indigo-500 hover:text-indigo-300"
                            onClick={() => saveCurrentFile()}
                            use:tippy={{
                                props: {
                                    content: "Save file",
                                    placement: "left",
                                }
                            }}
                        >
                            <svg xmlns="http://www.w3.org/2000/svg"
                                width="16"
                                height="16"
                                fill="currentColor"
                                class="bi bi-floppy2-fill"
                                viewBox="0 0 16 16">
                                <path d="M12 2h-2v3h2z" />
                                <path d="M1.5 0A1.5 1.5 0 0 0 0 1.5v13A1.5 1.5 0 0 0 1.5 16h13a1.5 1.5 0 0 0 1.5-1.5V2.914a1.5 1.5 0 0 0-.44-1.06L14.147.439A1.5 1.5 0 0 0 13.086 0zM4 6a1 1 0 0 1-1-1V1h10v4a1 1 0 0 1-1 1zM3 9h10a1 1 0 0 1 1 1v5H2v-5a1 1 0 0 1 1-1" />
                            </svg>
                        </span>
                    </Show>
                </div>
            </div>
            <div
                id="tab-content"
                class="select-none flex flex-auto h-full overflow-hidden min-w-0 max-h-full min-h-0"
                classList={{ hidden: !layoutState().requestTabPanelVisible }}
            >
                <Show when={layoutState().requestTabIndex === 1}>
                    <div id="tab-params-panel" class="flex flex-auto h-full min-w-0 max-h-full min-h-0">
                        <Parameters />
                    </div>
                </Show>
                <Show when={layoutState().requestTabIndex === 2}>
                    <div id="tab-headers-panel" class="flex flex-auto h-full min-w-0 max-h-full min-h-0">
                        <Headers />
                    </div>
                </Show>
                <Show when={layoutState().requestTabIndex === 3}>
                    <div id="tab-auth-panel" class="flex flex-auto h-full min-w-0 max-h-full min-h-0">
                        <Auth />
                    </div>
                </Show>
                <Show when={layoutState().requestTabIndex === 4}>
                    <div id="tab-body-panel" class="flex flex-auto h-full min-w-0 max-h-full min-h-0">
                        <Body />
                    </div>
                </Show>
            </div>
        </div >
    );
}
