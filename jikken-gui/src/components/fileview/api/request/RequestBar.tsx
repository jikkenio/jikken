import { createSignal, Show } from "solid-js";
import { $editorState, makeRequest, updateRequest, type Request } from "../../../../stores/editorState";
import { EntityType, HttpVerb } from "../../../../stores/enum";
import { tippy } from '../../../TippySolid.tsx';

export const RequestBar = () => {

    tippy;

    const getDisplayUrl = (request: Request | undefined) => {
        let url = request?.url ?? "";
        let params = request?.params ?? [];
        if (params.length === 0) return url;

        return `${url}?${params.map((p) => `${p.param}=${p.value}`).join("&")}`;
    }

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile];
    let currentTestFile = currentFile.type === EntityType.Test ? editorState.testFiles[currentFile.index] : undefined;

    const [executing, setExecuting] = createSignal(false);
    const [request, setRequest] = createSignal(currentTestFile?.testFile.request);

    $editorState.subscribe((state) => {
        currentFile = state.files[state.currentFile];
        currentTestFile = currentFile.type === EntityType.Test ? state.testFiles[currentFile.index] : undefined;
        setRequest(currentTestFile?.testFile.request);
        setExecuting(currentTestFile?.executing ?? false);
    });

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
                        value={request()?.method ?? HttpVerb.GET}
                        onChange={(e) => updateMethod(e.currentTarget.value)}
                    >
                        <option value={HttpVerb.GET}>GET</option>
                        <option value={HttpVerb.POST}>POST</option>
                        <option value={HttpVerb.PUT}>PUT</option>
                        <option value={HttpVerb.PATCH}>PATCH</option>
                        <option value={HttpVerb.DELETE}>DELETE</option>
                    </select>
                </div>
                <input
                    type="text"
                    id="url-input"
                    class="grow border-0 bg-transparent py-1.5 pl-1 text-neutral-300 text-sm placeholder:text-neutral-500 focus:ring-0"
                    placeholder="https://api.jikken.io"
                    value={getDisplayUrl(request()) || ""}
                    onChange={(e) => updateUrl(e.currentTarget.value)}
                />
            </div>
            <Show when={!executing()}>
                <button
                    id="send-button"
                    onClick={makeRequest}
                    disabled={!request()?.url}
                    class="flex-none w-20 rounded-[4px] bg-indigo-600 px-3 py-2 mx-2.5 text-sm font-semibold text-white shadow-sm disabled:bg-neutral-600 hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                >
                    Send
                </button>
            </Show>
            <Show when={executing()}>
                <button
                    id="executing-button"
                    class="flex-none w-20 rounded-[4px] px-3 py-2 mx-2.5 text-sm font-semibold text-white shadow-sm bg-neutral-600 cursor-default"
                    use:tippy={{
                        props: {
                            content: "Request in progress...",
                            placement: "bottom",
                        }
                    }}
                >
                    <svg class="mx-auto w-5 h-5 animate-spin"
                        viewBox="0 0 100 101"
                        fill="none"
                        xmlns="http://www.w3.org/2000/svg">
                        <path d="M100 50.5908C100 78.2051 77.6142 100.591 50 100.591C22.3858 100.591 0 78.2051 0 50.5908C0 22.9766 22.3858 0.59082 50 0.59082C77.6142 0.59082 100 22.9766 100 50.5908ZM9.08144 50.5908C9.08144 73.1895 27.4013 91.5094 50 91.5094C72.5987 91.5094 90.9186 73.1895 90.9186 50.5908C90.9186 27.9921 72.5987 9.67226 50 9.67226C27.4013 9.67226 9.08144 27.9921 9.08144 50.5908Z" fill="currentColor"></path>
                        <path d="M93.9676 39.0409C96.393 38.4038 97.8624 35.9116 97.0079 33.5539C95.2932 28.8227 92.871 24.3692 89.8167 20.348C85.8452 15.1192 80.8826 10.7238 75.2124 7.41289C69.5422 4.10194 63.2754 1.94025 56.7698 1.05124C51.7666 0.367541 46.6976 0.446843 41.7345 1.27873C39.2613 1.69328 37.813 4.19778 38.4501 6.62326C39.0873 9.04874 41.5694 10.4717 44.0505 10.1071C47.8511 9.54855 51.7191 9.52689 55.5402 10.0491C60.8642 10.7766 65.9928 12.5457 70.6331 15.2552C75.2735 17.9648 79.3347 21.5619 82.5849 25.841C84.9175 28.9121 86.7997 32.2913 88.1811 35.8758C89.083 38.2158 91.5421 39.6781 93.9676 39.0409Z" fill="#666"></path>
                    </svg>
                </button>
            </Show>
        </div>);
}
