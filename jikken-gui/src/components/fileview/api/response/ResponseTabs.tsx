import { createSignal, For, Show } from "solid-js";
import { $layoutState, setResponseTabActive } from "../../../../stores/layoutState";
import { Headers } from "./Headers";
import { Body } from './Body';
import { $editorState } from "../../../../stores/editorState";
import { EntityType } from "../../../../stores/enum";

export const ResponseTabs = () => {

    enum StatusType {
        SUCCESS,
        WARN,
        FAIL,
    };

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile];
    let currentTestFile = currentFile.type === EntityType.Test ? editorState.testFiles[currentFile.index] : undefined;

    let [response, setResponse] = createSignal(currentTestFile?.response);

    $editorState.subscribe((state) => {
        let currentFile = state.files[state.currentFile];
        let currentTestFile = currentFile.type === EntityType.Test ? state.testFiles[currentFile.index] : undefined;
        setResponse(currentTestFile?.response);
    });

    let [layout, setLayout] = createSignal($layoutState.get());

    $layoutState.subscribe((value) => {
        setLayout({ ...value });
    });

    const formatSize = () => {
        if (response()?.size === undefined) return "";

        let size = response()!.size!;
        if (size < 1000) {
            return `${size} bytes`;
        }

        if (size < 1e6) {
            return `${(size / 1000).toLocaleString("en-US", { maximumFractionDigits: 2, minimumFractionDigits: 0 })} KB`;
        }

        if (size < 1e9) {
            return `${(size / 1e6).toLocaleString("en-US", { maximumFractionDigits: 2, minimumFractionDigits: 0 })} MB`;
        }

        return `${(size / 1e9).toLocaleString("en-US", { maximumFractionDigits: 2, minimumFractionDigits: 0 })} GB`;
    };

    const formatStatus = () => {
        if (!response() || !response()?.status) return "";

        switch (response()!.status!) {
            case 200:
                return ["200 (OK)", StatusType.SUCCESS];
            case 201:
                return ["201 (Created)", StatusType.SUCCESS];
            case 204:
                return ["204 (No Content)", StatusType.SUCCESS];
            case 301:
                return ["301 (Moved Permanently)", StatusType.WARN];
            case 302:
                return ["302 (Found)", StatusType.WARN];
            case 307:
                return ["307 (Temporary Redirect)", StatusType.WARN];
            case 308:
                return ["308 (Permanent Redirect)", StatusType.WARN];
            case 400:
                return ["400 (Bad Request)", StatusType.FAIL];
            case 401:
                return ["401 (Unauthorized)", StatusType.FAIL];
            case 403:
                return ["403 (Forbidden)", StatusType.FAIL];
            case 404:
                return ["404 (Not Found)", StatusType.FAIL];
            case 500:
                return ["500 (Internal Server Error)", StatusType.FAIL];
            case 504:
                return ["504 (Gateway Timeout)", StatusType.FAIL];
            default:
                return [response()?.status.toString(), StatusType.FAIL];
        }
    };

    const formatTime = () => {
        let time = response()?.time;
        if (!time) return "";
        if (time < 1000) return `${time} ms`;

        if (time < 60000) {
            let ms = time % 1000;
            let s = (time - ms) / 1000;
            return `${s}s ${ms}ms`;
        }

        let remainder = time % 60000;
        let m = (time - remainder) / 60000;
        let s = ((time - (m * 60000)) / 1000).toLocaleString("en-US", { maximumFractionDigits: 2, minimumFractionDigits: 0 });
        return `${m}m ${s}s`;
    }

    return (
        <div
            class="flex flex-auto flex-col h-full min-w-0"
            classList={{
                hidden: !response()
            }}>
            <div class="border-b border-neutral-800 m-3 mt-0 flex justify-between flex-shrink-0">
                <nav class="-mb-px flex space-x-4" aria-label="Tabs">
                    <For each={layout().responseTabs}>
                        {(tab) => (
                            <div
                                class="group flex min-w-12 justify-center whitespace-nowrap border-b-2 py-2 text-sm cursor-pointer font-medium"
                                classList={{
                                    "border-indigo-500 text-neutral-300": tab.index === layout().responseTabIndex,
                                    "border-transparent text-neutral-400 hover:text-neutral-200": tab.index !== layout().responseTabIndex
                                }}
                                onClick={() => setResponseTabActive(tab.index)}
                            >
                                {tab.label}

                                <Show when={tab.showCount && tab.items > 0}>
                                    <span
                                        class="ml-1.5 my-auto rounded-[4px] px-[5px] py-px text-xs font-medium inline-block"
                                        classList={{
                                            "bg-indigo-500 text-white": tab.index === layout().responseTabIndex,
                                            "bg-neutral-850 ring-1 ring-inset ring-neutral-600 text-neutral-300 group-hover:text-black group-hover:bg-neutral-300 group-hover:ring-0": tab.index !== layout().responseTabIndex,
                                        }}
                                    >
                                        {tab.items}
                                    </span>
                                </Show>

                                <Show when={!tab.showCount && tab.items > 0}>
                                    <span
                                        class="ml-0.5 my-auto inline-block"
                                        classList={{
                                            "text-indigo-500": tab.index === layout().responseTabIndex,
                                            "text-neutral-300 group-hover:text-neutral-300": tab.index !== layout().responseTabIndex,
                                        }}
                                    >
                                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-check" viewBox="0 0 16 16">
                                            <path d="M10.97 4.97a.75.75 0 0 1 1.07 1.05l-3.99 4.99a.75.75 0 0 1-1.08.02L4.324 8.384a.75.75 0 1 1 1.06-1.06l2.094 2.093 3.473-4.425z" />
                                        </svg>
                                    </span>
                                </Show>
                            </div>
                        )}
                    </For>
                </nav>

                <Show when={response()}>
                    <div class="flex space-x-2 items-center text-xs text-neutral-400 mr-2">
                        <span classList={{
                            "text-emerald-500/80": formatStatus()[1] === StatusType.SUCCESS,
                            "text-rose-500/70": formatStatus()[1] === StatusType.FAIL,
                            "text-amber-400/70": formatStatus()[1] === StatusType.WARN,
                        }}>
                            {formatStatus()[0]}
                        </span>
                        <span class="text-neutral-600">|</span>
                        <span>{formatTime()}</span>
                        <span class="text-neutral-600">|</span>
                        <span>{formatSize()}</span>
                    </div>
                </Show>
            </div>

            <div id="tab-content"
                class="select-none flex flex-auto h-full overflow-hidden min-w-0"
                classList={{ hidden: !layout().responseTabPanelVisible }}
            >
                <Show when={layout().responseTabIndex === 1}>
                    <div id="tab-body-panel" class="flex flex-auto h-full min-w-0">
                        <Body />
                    </div>
                </Show>
                <Show when={layout().responseTabIndex === 2}>
                    <div id="tab-headers-panel" class="w-full">
                        <Headers />
                    </div>
                </Show>
            </div>
        </div>
    );
};