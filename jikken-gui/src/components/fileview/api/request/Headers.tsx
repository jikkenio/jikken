import { setRequestTabCount } from '../../../../stores/layoutState';
import { createSignal, For, Show } from 'solid-js';
import { $editorState, updateAuth, updateRequest, type HttpHeader, type Request } from '../../../../stores/editorState';
import { parseBasicAuthHeader, type AuthState } from '../../../../stores/authState';
import { AuthType, EntityType } from '../../../../stores/enum';
import { tippy } from '../../../TippySolid.tsx';

export const Headers = () => {

    tippy

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile];
    let currentTestFile = currentFile.type === EntityType.Test ? editorState.testFiles[currentFile.index] : undefined;

    const [headers, setHeaders] = createSignal<HttpHeader[]>((() => {
        const headers = currentTestFile?.testFile.request?.headers ?? [];
        return [...headers, { header: "", value: "", generated: false }];
    })());

    $editorState.subscribe((state) => {
        currentFile = state.files[state.currentFile];
        currentTestFile = currentFile.type === EntityType.Test ? state.testFiles[currentFile.index] : undefined;
        const headers = currentTestFile?.testFile.request?.headers ?? [];
        setHeaders([...headers, { header: "", value: "", generated: false }]);
    });


    const onHeaderInput = (index: number) => {
        let currentHeaders = headers();

        // if the last row is not empty, add another row
        if (index === currentHeaders.length - 1) {
            setHeaders([...currentHeaders, { header: "", value: "", generated: false }]);
            setRequestTabCount("tab-headers", currentHeaders.length);
        }
    };

    const onHeaderChange = (index: number, header: HttpHeader) => {
        console.log(`saving header at index ${index}`);
        let currentHeaders = [...headers()];
        currentHeaders[index] = header;

        // if we emptied a row and it's not the last, delete it
        if (header.header === "" && header.value === "" && index < currentHeaders.length - 1) {
            deleteHeader(index);
            return;
        }

        if (header.header.toLocaleLowerCase() === "authorization") {
            let auth: AuthState = { type: AuthType.None };
            if (header.value.startsWith("Bearer ")) {
                auth.type = AuthType.Bearer;
                auth.data = {
                    token: header.value.split(" ")[1]
                };
            } else if (header.value.startsWith("Basic")) {
                auth.type = AuthType.Basic;
                let data = parseBasicAuthHeader(header);
                auth.data = {
                    username: data[0],
                    password: data[1],
                };
            }

            updateAuth(auth);
        }

        updateHeaders(currentHeaders);
    }

    const deleteHeader = (index: number) => {
        console.log(`deleting header at ${index}`);
        let currentHeaders = [...headers()];
        let deleted = currentHeaders.splice(index, 1)[0];
        if (deleted.header.toLocaleLowerCase() === "authorization") {
            updateAuth({ type: AuthType.None });
        }

        setHeaders(currentHeaders);
        setRequestTabCount("tab-headers", currentHeaders.length - 1);
        updateHeaders(currentHeaders);
    };

    const updateHeaders = (headers: HttpHeader[]) => {
        let request = { ...currentTestFile?.testFile.request ?? {} as Request };
        headers.splice(-1, 1);
        request.headers = headers;
        updateRequest(request);
    }

    return (
        <div id="tab-headers-panel" class="p-3 pt-1 w-full flex flex-col">
            <div class="text-neutral-400 mb-2 text-sm font-medium">Headers</div>

            <ul class="overflow-y-auto">
                <For each={headers()}>
                    {(header, index) => (
                        <li class="flex flex-row group text-neutral-300">
                            <div class="flex flex-row flex-1 items-center border border-1 border-b-0 group-last:border-b border-neutral-700 focus-within:ring-2 focus-within:ring-inset focus-within:ring-indigo-600">
                                <input placeholder="Key"
                                    spellcheck={false}
                                    autocorrect="off"
                                    value={header.header}
                                    onInput={(_) => onHeaderInput(index())}
                                    onChange={(e) => onHeaderChange(index(), { header: e.currentTarget.value, value: header.value, generated: false })}
                                    class="grow text-sm bg-transparent pl-2 p-1 border-0 placeholder:text-neutral-500 focus-within:ring-0"
                                />
                                <Show when={header.generated}>
                                    <span class="flex-none p-1 pr-1.5 text-neutral-400 hover:text-indigo-400"
                                        use:tippy={{
                                            props: {
                                                content: "Auto-generated value",
                                            }
                                        }}>
                                        <svg xmlns="http://www.w3.org/2000/svg"
                                            width="12"
                                            height="12"
                                            fill="currentColor"
                                            class="bi bi-lightning-charge"
                                            viewBox="0 0 16 16">
                                            <path d="M11.251.068a.5.5 0 0 1 .227.58L9.677 6.5H13a.5.5 0 0 1 .364.843l-8 8.5a.5.5 0 0 1-.842-.49L6.323 9.5H3a.5.5 0 0 1-.364-.843l8-8.5a.5.5 0 0 1 .615-.09zM4.157 8.5H7a.5.5 0 0 1 .478.647L6.11 13.59l5.732-6.09H9a.5.5 0 0 1-.478-.647L9.89 2.41z" />
                                        </svg>
                                    </span>
                                </Show>
                            </div>
                            <input placeholder="Value"
                                spellcheck={false}
                                autocorrect="off"
                                value={header.value}
                                onInput={(_) => onHeaderInput(index())}
                                onChange={(e) => onHeaderChange(index(), { header: header.header, value: e.currentTarget.value, generated: false })}
                                class="flex-1 text-sm bg-transparent pl-2 p-1 border-1 border-l-0 border-b-0 group-last:border-b border-neutral-700 placeholder:text-neutral-500 focus-within:ring-2 focus-within:ring-inset focus-within:ring-indigo-600"
                            />
                            <div
                                class="flex-none w-8 p-2 text-neutral-500 cursor-pointer group-last:cursor-default group-last:pointer-events-none hover:text-red-500"
                                onClick={[deleteHeader, index()]}
                            >
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    width="16"
                                    height="16"
                                    fill="currentColor"
                                    class="bi bi-trash-fill invisible group-hover:visible group-last:group-hover:invisible"
                                    viewBox="0 0 16 16"
                                >
                                    <path
                                        d="M2.5 1a1 1 0 0 0-1 1v1a1 1 0 0 0 1 1H3v9a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2V4h.5a1 1 0 0 0 1-1V2a1 1 0 0 0-1-1H10a1 1 0 0 0-1-1H7a1 1 0 0 0-1 1zm3 4a.5.5 0 0 1 .5.5v7a.5.5 0 0 1-1 0v-7a.5.5 0 0 1 .5-.5M8 5a.5.5 0 0 1 .5.5v7a.5.5 0 0 1-1 0v-7A.5.5 0 0 1 8 5m3 .5v7a.5.5 0 0 1-1 0v-7a.5.5 0 0 1 1 0"
                                    ></path>
                                </svg>
                            </div>
                        </li>
                    )}
                </For>
            </ul>
        </div>
    );
}
