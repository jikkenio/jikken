import { createSignal, Show } from 'solid-js';
import { setRequestTabCount } from '../../../../stores/layoutState';
import { $editorState, updateRequest, updateAuth, updateCompare, type HttpHeader, type Request, type Compare, updateCompareState } from '../../../../stores/editorState';
import { type AuthState, type BasicAuthData, type BearerAuthData } from '../../../../stores/authState';
import { AuthType, EntityType } from '../../../../stores/enum';
import { tippy } from '../../../TippySolid';

export const Auth = () => {

    tippy;

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile];
    let currentTestFile = currentFile.type === EntityType.Test ? editorState.testFiles[currentFile.index] : undefined;

    const [auth, setAuth] = createSignal(currentTestFile?.auth);
    const [inheritCompare, setInheritCompare] = createSignal(currentTestFile?.compare?.inheritAuth ?? false);

    $editorState.subscribe((state) => {
        currentFile = state.files[state.currentFile];
        currentTestFile = currentFile.type === EntityType.Test ? state.testFiles[currentFile.index] : undefined;
        setAuth(currentTestFile?.auth);
        setInheritCompare(currentTestFile?.compare?.inheritAuth ?? false);
    });

    const onTypeChange = (index: number, compare: boolean = false) => {
        let currentAuth = auth()!;
        let data = {
            type: AuthType[AuthType[index] as keyof typeof AuthType],
        };

        if (compare) {
            currentAuth.compare = data;
        } else {
            currentAuth.request = data;
        }

        updateAuth(currentAuth);
        setRequestTabCount("tab-auth", index !== AuthType.None ? 1 : 0);
        resolveHeaders(currentAuth, compare);
    };

    const onBasicAuthFieldChange = (field: string, value: string, compare: boolean = false) => {
        let currentAuth = auth()!;
        let data;
        if (compare) {
            data = (currentAuth.compare?.data ?? {}) as BasicAuthData;
        } else {
            data = (currentAuth.request?.data ?? {}) as BasicAuthData;
        }

        if (field === "username") {
            data.username = value;
        } else {
            data.password = value;
        }

        if (compare) {
            currentAuth.compare!.data = { ...data };
        } else {
            currentAuth.request.data = { ...data };
        }

        updateAuth({ ...currentAuth });
        resolveHeaders(currentAuth, compare);
    };

    const onBearerAuthTokenChange = (token: string, compare: boolean = false) => {
        let currentAuth = auth()!;

        if (compare) {
            currentAuth.compare!.data = { token: token } as BearerAuthData;
        } else {
            currentAuth.request.data = { token: token } as BearerAuthData;
        }

        updateAuth({ ...currentAuth });
        resolveHeaders({ ...currentAuth }, compare);
    };

    const resolveHeaders = (auth: AuthState, compare: boolean = false) => {
        const file = currentTestFile!.testFile;
        let headers = (compare ? file?.compare?.headers : file?.request?.headers) ?? [];

        switch (auth.request.type) {
            case AuthType.None:
                removeHeader("Authorization", headers);
                break;
            case AuthType.Basic:
                let basicData = (compare ? auth.compare?.data : auth.request.data) as BasicAuthData;
                if (basicData && basicData.username && basicData.password) {
                    let value = `${basicData.username}:${basicData.password}`;
                    addHeader({ header: "Authorization", value: `Basic ${btoa(value)}`, generated: true }, headers);
                } else {
                    removeHeader("Authorization", headers);
                }
                break;
            case AuthType.Bearer:
                let bearerData = (compare ? auth.compare?.data : auth.request.data) as BearerAuthData;
                if (bearerData && bearerData.token) {
                    addHeader({ header: "Authorization", value: `Bearer ${bearerData.token}`, generated: true }, headers);
                } else {
                    removeHeader("Authorization", headers);
                }
                break;
            default:
        }

        updateHeaders(headers, compare);
    };

    const addHeader = (header: HttpHeader, headers: HttpHeader[]) => {
        let existing = headers.find(h => h.header.toLowerCase() === header.header.toLowerCase());
        if (existing) {
            existing.value = header.value;
        } else {
            headers.push(header);
        }
    };

    const removeHeader = (name: string, headers: HttpHeader[]) => {
        let index = headers.findIndex(h => h.header.toLowerCase() === name.toLowerCase());
        if (index > -1) {
            headers.splice(index, 1);
        }
    };

    const updateHeaders = (headers: HttpHeader[], compare: boolean = false) => {
        const file = currentTestFile!.testFile;

        if (compare) {
            let compare = { ...file?.compare ?? {} as Compare };
            compare.headers = headers;
            updateCompare(compare);
        } else {
            let request = { ...file?.request ?? {} as Request };
            request.headers = headers;
            updateRequest(request);
        }

        setRequestTabCount("tab-headers", headers.length);
    };

    const toggleInherit = () => {
        let value = !inheritCompare();
        setInheritCompare(value);

        let compare = currentTestFile!.compare!;
        compare.inheritAuth = value;
        updateCompareState({ ...compare });
    };

    const getCompareAuth = () => {
        let currentAuth = auth();
        if (inheritCompare()) return currentAuth?.request;
        return currentAuth?.compare;
    };

    return (
        <div id="tab-auth-panel" class="w-full grid grid-cols-2 divide-x-[0.5px] divide-neutral-700">

            <div class="p-3"
                classList={{
                    "col-span-2": auth()?.compare === undefined,
                    "col-span-1": auth()?.compare !== undefined
                }}>
                <div class="mb-4">
                    <label for="type-select" class="text-xs text-neutral-400 mr-3">Auth Type</label>
                    <div class="relative inline-block">
                        <select id="type-select" class="h-7 w-32 p-0 pl-2 pr-7 bg-transparent text-xs text-neutral-300 border border-neutral-700 rounded-[3px] focus:ring-0 focus:border-neutral-500 appearance-none outline-none"
                            onChange={(e) => onTypeChange(+e.currentTarget.value)} value={auth()?.request.type ?? AuthType.None}>
                            <option selected value={AuthType.None}>None</option>
                            <option value={AuthType.Basic}>Basic Auth</option>
                            <option value={AuthType.Bearer}>Bearer Token</option>
                        </select>
                    </div>
                </div>

                <Show when={auth()?.request.type === AuthType.Basic}>
                    <div class="w-full max-w-72 inline-grid grid-cols-3 gap-y-2">
                        <div class="flex items-center">
                            <label for="username-input" class="text-sm text-neutral-400">Username</label>
                        </div>
                        <input id="username-input"
                            spellcheck={false}
                            autocorrect="off"
                            placeholder="username"
                            value={(auth()?.request.data as BasicAuthData)?.username ?? ""}
                            class="col-span-2 h-7 w-36 p-1 px-2 form-input rounded-[3px] bg-transparent text-sm text-neutral-300 border-neutral-600 focus:ring-0 focus:border-neutral-400 placeholder:text-neutral-500"
                            onChange={(e) => onBasicAuthFieldChange("username", e.currentTarget.value)} />
                        <div class="flex items-center">
                            <label for="password-input" class="text-sm text-neutral-400">Password</label>
                        </div>
                        <input id="password-input" type="password" placeholder="password" value={(auth()?.request.data as BasicAuthData)?.password ?? ""}
                            class="col-span-2 h-7 w-36 p-1 px-2 rounded-[3px] bg-transparent text-sm text-neutral-300 border-neutral-600 focus:ring-0 focus:border-neutral-400 placeholder:text-neutral-500"
                            onChange={(e) => onBasicAuthFieldChange("password", e.currentTarget.value)} />
                    </div>
                </Show>

                <Show when={auth()?.request.type === AuthType.Bearer}>
                    <div class="w-full max-w-72 inline-grid grid-cols-3 gap-y-2">
                        <div class="flex items-center">
                            <label for="token-input" class="text-sm text-neutral-400">Token</label>
                        </div>
                        <input id="token-input"
                            spellcheck={false}
                            autocorrect="off"
                            placeholder="token"
                            value={(auth()?.request.data as BearerAuthData)?.token ?? ""}
                            class="col-span-2 h-7 p-0 px-2 rounded-[3px] bg-transparent text-sm text-neutral-300 border-neutral-600 focus:ring-0 focus:border-neutral-400 placeholder:text-neutral-500"
                            onChange={(e) => onBearerAuthTokenChange(e.currentTarget.value)} />
                    </div>
                </Show>
            </div>

            <Show when={auth()?.compare}>
                <div class="col-span-1 p-3">
                    <div class="flex grid grid-cols-2">
                        <div class="col-span-1 flex flex-row items-center mb-4">
                            <label for="compare-type-select" class="text-xs text-neutral-400 mr-3">Auth Type</label>
                            <select id="compare-type-select" disabled={inheritCompare()}
                                class="h-7 w-32 p-0 pl-2 bg-transparent text-xs text-neutral-300 border border-neutral-700 rounded-[3px] focus:ring-0 focus:border-neutral-500 appearance-none outline-none disabled:text-neutral-500"
                                onChange={(e) => onTypeChange(+e.currentTarget.value, true)} value={getCompareAuth()?.type ?? AuthType.None}>
                                <option selected value={AuthType.None}>None</option>
                                <option value={AuthType.Basic}>Basic Auth</option>
                                <option value={AuthType.Bearer}>Bearer Token</option>
                            </select>
                        </div>
                        <div class="col-span-1 flex justify-end">
                            <div class="group cursor-pointer mt-[-10px]"
                                use:tippy={{
                                    props: {
                                        content: "Inherit from Request 1"
                                    }
                                }}
                            >
                                <input id="inherit-checkbox"
                                    type="checkbox"
                                    checked={inheritCompare()}
                                    onChange={toggleInherit}
                                    class="peer size-3 bg-transparent border-neutral-600 rounded-[2px] cursor-pointer checked:bg-indigo-600 group-hover:border-neutral-200 checked:border-transparent" />
                                <label for="inherit-checkbox" class="text-neutral-400 h-4 text-xs cursor-pointer ml-1.5 group-hover:text-neutral-200 peer-checked:text-neutral-300">Inherit</label>
                            </div>
                        </div>
                    </div>

                    <Show when={getCompareAuth()?.type === AuthType.Basic}>
                        <div class="w-full max-w-72 inline-grid grid-cols-3 gap-y-2">
                            <div class="flex items-center">
                                <label for="compare-username-input" class="text-sm text-neutral-400">Username</label>
                            </div>
                            <input id="compare-username-input"
                                disabled={inheritCompare()}
                                spellcheck={false}
                                autocorrect="off"
                                placeholder="username"
                                value={(getCompareAuth()?.data as BasicAuthData)?.username ?? ""}
                                class="col-span-2 h-7 w-36 p-1 px-2 form-input rounded-[3px] bg-transparent text-sm text-neutral-300 border-neutral-600 focus:ring-0 focus:border-neutral-400 placeholder:text-neutral-500 disabled:text-neutral-500"
                                onChange={(e) => onBasicAuthFieldChange("username", e.currentTarget.value, true)} />
                            <div class="flex items-center">
                                <label for="compare-password-input" class="text-sm text-neutral-400">Password</label>
                            </div>
                            <input id="compare-password-input"
                                type="password"
                                disabled={inheritCompare()}
                                placeholder="password"
                                value={(getCompareAuth()?.data as BasicAuthData)?.password ?? ""}
                                class="col-span-2 h-7 w-36 p-1 px-2 rounded-[3px] bg-transparent text-sm text-neutral-300 border-neutral-600 focus:ring-0 focus:border-neutral-400 placeholder:text-neutral-500 disabled:text-neutral-500"
                                onChange={(e) => onBasicAuthFieldChange("password", e.currentTarget.value, true)} />
                        </div>
                    </Show>

                    <Show when={getCompareAuth()?.type === AuthType.Bearer}>
                        <div class="w-full max-w-72 inline-grid grid-cols-3 gap-y-2">
                            <div class="flex items-center">
                                <label for="compare-token-input" class="text-sm text-neutral-400">Token</label>
                            </div>
                            <input id="compare-token-input"
                                disabled={inheritCompare()}
                                spellcheck={false}
                                autocorrect="off"
                                placeholder="token"
                                value={(getCompareAuth()?.data as BearerAuthData)?.token ?? ""}
                                class="col-span-2 h-7 p-0 px-2 rounded-[3px] bg-transparent text-sm text-neutral-300 border-neutral-600 focus:ring-0 focus:border-neutral-400 placeholder:text-neutral-500 disabled:text-neutral-500"
                                onChange={(e) => onBearerAuthTokenChange(e.currentTarget.value, true)} />
                        </div>
                    </Show>
                </div>
            </Show>
        </div>
    );
};
