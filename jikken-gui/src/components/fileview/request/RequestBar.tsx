import { createSignal } from "solid-js";
import { $editorState, HttpVerb, makeRequest, updateRequest, type Request } from "../../../stores/editorState";

export const RequestBar = () => {

    const getDisplayUrl = (request: Request | undefined) => {
        let url = request?.url ?? "";
        let params = request?.params ?? [];
        if (params.length === 0) return url;

        return `${url}?${params.map((p) => `${p.param}=${p.value}`).join("&")}`;
    }

    let editorStore = $editorState.get()
    let request = editorStore.files[editorStore.currentFile].testFile.request;
    const [url, setUrl] = createSignal(request?.url);
    const [displayUrl, setDisplayUrl] = createSignal(getDisplayUrl(request));
    const [method, setMethod] = createSignal(editorStore.files[editorStore.currentFile].testFile.request?.method ?? HttpVerb.GET)

    $editorState.subscribe((newState) => {
        let newRequest = newState.files[newState.currentFile].testFile.request;
        setUrl(newRequest?.url);
        setDisplayUrl(getDisplayUrl(newRequest));
        setMethod(newRequest?.method ?? HttpVerb.GET);
        request = newRequest;
    });

    const updateMethod = (value: string) => {
        let currentRequest = request || {};
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
        let currentRequest = request || {};
        currentRequest.url = value.split("?")[0];
        updateParamsFromUrl(value, currentRequest);
        updateRequest(currentRequest);
    }

    return (
        <div class="mt-2 p-2 flex">
            <div
                class="flex grow rounded-[4px] shadow-sm ring-1 ring-inset ring-neutral-500 focus-within:ring-neutral-300"
            >
                <select
                    class="flex-none select-none items-center pl-3 text-white text-sm border-none bg-transparent focus:ring-0"
                    value={method()}
                    onChange={(e) => updateMethod(e.currentTarget.value)}
                >
                    <option value={HttpVerb.GET}>GET</option>
                    <option value={HttpVerb.POST}>POST</option>
                    <option value={HttpVerb.PUT}>PUT</option>
                    <option value={HttpVerb.PATCH}>PATCH</option>
                    <option value={HttpVerb.DELETE}>DELETE</option>
                </select>
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
