import { $editorState, makeRequest, updateRequest, type Request } from "../../../../stores/editorState";
import { EntityType, HttpVerb } from "../../../../stores/enum";
import { useStore } from "@nanostores/solid";

export const RequestBar = () => {

    const editorState = useStore($editorState);

    const getDisplayUrl = (request: Request | undefined) => {
        let url = request?.url ?? "";
        let params = request?.params ?? [];
        if (params.length === 0) return url;

        return `${url}?${params.map((p) => `${p.param}=${p.value}`).join("&")}`;
    }

    const currentFile = () => editorState().files[editorState().currentFile];
    const currentTestFile = () => currentFile().type === EntityType.Test ? editorState().testFiles[currentFile().index] : undefined;
    const request = () => currentTestFile()?.testFile.request;
    const url = () => request()?.url;
    const displayUrl = () => getDisplayUrl(request());
    const method = () => request()?.method ?? HttpVerb.GET;

    const updateMethod = (value: string) => {
        let currentRequest = request() || {};
        currentRequest.method = value as HttpVerb;
        updateRequest(currentRequest);
    }

    const updateParamsFromUrl = (newUrl: string, request: Request) => {
        request.params = []
        if (!newUrl) return;

        let urlParts = newUrl.split("?");
        if (urlParts.length < 2) return;

        let paramStrings = urlParts[1].split("&");
        for (var paramString of paramStrings) {
            let parts = paramString.split("=");
            request.params.push({
                param: parts[0],
                value: parts.length > 1 ? parts[1] : "",
                generated: false,
            });
        }
    }

    const updateUrl = (value: string) => {
        let currentRequest = request() || {};
        currentRequest.url = value.split("?")[0];
        updateParamsFromUrl(value, currentRequest);
        updateRequest(currentRequest);
    }

    return (
        <div class="mt-2 p-2 flex">
            <div
                class="flex grow rounded-[4px] shadow-sm ring-1 ring-inset ring-neutral-500 focus-within:ring-neutral-300"
            >
                <div class="relative flex items-center">
                    <select
                        class="flex-none select-none items-center pl-3 pr-8 text-white text-sm border-none bg-transparent focus:ring-0 appearance-none outline-none"
                        style="box-shadow: none;"
                        value={method()}
                        onChange={(e) => updateMethod(e.currentTarget.value)}
                    >
                        <option value={HttpVerb.GET}>GET</option>
                        <option value={HttpVerb.POST}>POST</option>
                        <option value={HttpVerb.PUT}>PUT</option>
                        <option value={HttpVerb.PATCH}>PATCH</option>
                        <option value={HttpVerb.DELETE}>DELETE</option>
                    </select>
                    <svg class="absolute right-2 pointer-events-none h-4 w-4 text-neutral-400" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
                        <path fill-rule="evenodd" d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z" clip-rule="evenodd" />
                    </svg>
                </div>
                <input
                    type="text"
                    id="url-input"
                    class="grow border-0 bg-transparent py-1.5 pl-1 text-neutral-300 text-sm placeholder:text-neutral-500 focus:ring-0"
                    placeholder="https://api.jikken.io"
                    value={displayUrl() || ""}
                    onChange={(e) => updateUrl(e.currentTarget.value)}
                />
            </div>
            <button
                id="send-button"
                onClick={makeRequest}
                disabled={!url()}
                class="flex-none w-20 rounded-[4px] bg-indigo-600 px-3 py-2 mx-2.5 text-sm font-semibold text-white shadow-sm disabled:bg-neutral-600 hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
            >
                Send
            </button>
        </div>);
}
