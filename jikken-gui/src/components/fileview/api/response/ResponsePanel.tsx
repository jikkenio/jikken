import { createSignal, For, Show } from "solid-js";
import { $layoutState, setResponseCompareTabActive } from "../../../../stores/layoutState";
import { $editorState } from "../../../../stores/editorState";
import { EntityType } from "../../../../stores/enum";
import { ResponseTabs } from "./ResponseTabs";
import MonacoDiffViewerSolid from "../MonacoDiffViewerSolid";

export const ResponsePanel = () => {

    enum StatusType {
        SUCCESS,
        WARN,
        FAIL,
    };

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile];
    let currentTestFile = currentFile.type === EntityType.Test ? editorState.testFiles[currentFile.index] : undefined;

    let [showCompare, setShowCompare] = createSignal(currentTestFile?.testFile.compare !== undefined);
    let [response, setResponse] = createSignal(currentTestFile?.responses.request);
    let [compareResponse, setCompareResponse] = createSignal(currentTestFile?.responses.compare);

    $editorState.subscribe((state) => {
        let file = state.files[state.currentFile];
        let testFile = file.type === EntityType.Test ? state.testFiles[file.index] : undefined;
        setShowCompare(testFile?.testFile.compare !== undefined);
        setResponse(testFile?.responses.request);
        setCompareResponse(testFile?.responses.compare);
    });

    let [layout, setLayout] = createSignal($layoutState.get());

    $layoutState.subscribe((value) => {
        setLayout({ ...value });
    });

    const prettyJson = (body?: string) => {
        if (!body) return undefined;
        try {
            return JSON.stringify(JSON.parse(body), null, 2);
        } catch {
            // If JSON parsing fails, return as-is
            return body;
        }
    };

    const formatSize = (maybeSize?: number) => {
        if (maybeSize === undefined) return "";

        let size = maybeSize!;
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

    const formatStatus = (status?: number) => {
        if (!status) return "";

        switch (status!) {
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
                return [status.toString(), StatusType.FAIL];
        }
    };

    const formatTime = (time?: number) => {
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
    };

    return (
        <div class="flex flex-auto flex-col size-full min-w-0 min-h-0 max-h-full"
            classList={{
                hidden: !response() && !compareResponse()
            }}>

            <Show when={!showCompare()}>
                <ResponseTabs compare={false} />
            </Show>

            <Show when={showCompare()}>
                <div class="border-b border-neutral-800 m-3 mt-0 flex justify-between flex-shrink-0">
                    <nav class="-mb-px flex space-x-4" aria-label="Tabs">
                        <For each={layout().responseCompareTabs}>
                            {(tab) => (
                                <div
                                    class="group flex min-w-12 justify-center whitespace-nowrap border-b-2 py-2 text-sm cursor-pointer font-medium"
                                    classList={{
                                        "border-indigo-500 text-neutral-300": tab.index === layout().responseCompareTabIndex,
                                        "border-transparent text-neutral-400 hover:text-neutral-200": tab.index !== layout().responseCompareTabIndex
                                    }}
                                    onClick={() => setResponseCompareTabActive(tab.index)}
                                >
                                    {tab.label}

                                    <Show when={tab.showCount && tab.items > 0}>
                                        <span
                                            class="ml-1.5 my-auto rounded-[4px] px-[5px] py-px text-xs font-medium inline-block"
                                            classList={{
                                                "bg-indigo-500 text-white": tab.index === layout().responseCompareTabIndex,
                                                "bg-neutral-850 ring-1 ring-inset ring-neutral-600 text-neutral-300 group-hover:text-black group-hover:bg-neutral-300 group-hover:ring-0": tab.index !== layout().responseCompareTabIndex,
                                            }}
                                        >
                                            {tab.items}
                                        </span>
                                    </Show>

                                    <Show when={!tab.showCount && tab.items > 0}>
                                        <span
                                            class="ml-1.5 my-auto inline-block"
                                            classList={{
                                                "text-indigo-500": tab.index === layout().responseCompareTabIndex,
                                                "text-neutral-400 group-hover:text-neutral-300": tab.index !== layout().responseCompareTabIndex,
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
                </div>

                <Show when={layout().responseCompareTabPanelVisible}>
                    <div class="flex flex-col flex-auto size-full min-w-0 min-h-0 max-h-full">
                        <Show when={layout().responseCompareTabIndex === 1}>
                            <div class="flex flex-row pt-1 px-3">
                                <Show when={response()}>
                                    <div class="flex flex-1 space-x-2 items-center text-xs text-neutral-400">
                                        <span classList={{
                                            "text-emerald-500/80": formatStatus(response()?.status)[1] === StatusType.SUCCESS,
                                            "text-rose-500/70": formatStatus(response()?.status)[1] === StatusType.FAIL,
                                            "text-amber-400/70": formatStatus(response()?.status)[1] === StatusType.WARN,
                                        }}>
                                            {formatStatus(response()?.status)[0]}
                                        </span>
                                        <span class="text-neutral-600">|</span>
                                        <span>{formatTime(response()?.time)}</span>
                                        <span class="text-neutral-600">|</span>
                                        <span>{formatSize(response()?.size)}</span>
                                    </div>
                                </Show>
                                <Show when={compareResponse()}>
                                    <div class="flex flex-1 space-x-2 justify-end items-center text-xs text-neutral-400">
                                        <span classList={{
                                            "text-emerald-500/80": formatStatus(compareResponse()?.status)[1] === StatusType.SUCCESS,
                                            "text-rose-500/70": formatStatus(compareResponse()?.status)[1] === StatusType.FAIL,
                                            "text-amber-400/70": formatStatus(compareResponse()?.status)[1] === StatusType.WARN,
                                        }}>
                                            {formatStatus(compareResponse()?.status)[0]}
                                        </span>
                                        <span class="text-neutral-600">|</span>
                                        <span>{formatTime(compareResponse()?.time)}</span>
                                        <span class="text-neutral-600">|</span>
                                        <span>{formatSize(compareResponse()?.size)}</span>
                                    </div>
                                </Show>
                            </div>
                            <div class="size-full flex-auto min-w-0 min-h-0 p-3">
                                <MonacoDiffViewerSolid language="json" values={[prettyJson(response()?.body), prettyJson(compareResponse()?.body)]} />
                            </div>
                        </Show>
                        <Show when={layout().responseCompareTabIndex === 2}>
                            <div class="flex size-full">
                                <ResponseTabs compare={false} />
                            </div>
                        </Show>
                        <Show when={layout().responseCompareTabIndex === 3}>
                            <div class="flex size-full">
                                <ResponseTabs compare={true} />
                            </div>
                        </Show>
                    </div>
                </Show>
            </Show>
        </div>
    );
};
