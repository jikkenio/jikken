import { createSignal, Show } from 'solid-js';
import { setRequestTabCount } from '../../../stores/layoutState';
import { $editorState, updateRequest, updateAuth, type HttpHeader, type Request } from '../../../stores/editorState';
import { AuthType, type AuthState, type BasicAuthData, type BearerAuthData } from '../../../stores/authState';

export const Auth = () => {

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile].testFile;

    let [auth, setAuth] = createSignal(editorState.files[editorState.currentFile].auth);

    $editorState.subscribe((state) => {
        currentFile = state.files[state.currentFile].testFile;
        setAuth(state.files[state.currentFile].auth);
    });

    const onTypeChange = (index: number) => {
        let auth = {
            type: AuthType[AuthType[index] as keyof typeof AuthType],
        };
        updateAuth(auth);
        setRequestTabCount("tab-auth", index !== AuthType.None ? 1 : 0);
        resolveHeaders(auth);
    };

    const onBasicAuthFieldChange = (field: string, value: string) => {
        let currentAuth = auth();
        let currentData = (currentAuth.data ?? {}) as BasicAuthData;
        if (field === "username") {
            currentData.username = value;
        } else {
            currentData.password = value;
        }

        currentAuth.data = { ...currentData }
        updateAuth({ ...currentAuth });
        resolveHeaders(currentAuth);
    };

    const onBearerAuthTokenChange = (token: string) => {
        let currentAuth = auth();
        currentAuth.data = { token: token } as BearerAuthData;
        updateAuth({ ...currentAuth });
        resolveHeaders(currentAuth);
    };

    const resolveHeaders = (auth: AuthState) => {
        let headers = currentFile.request?.headers ?? [];

        switch (auth.type) {
            case AuthType.None:
                removeHeader("Authorization", headers);
                break;
            case AuthType.Basic:
                let basicData = auth.data as BasicAuthData;
                if (basicData && basicData.username && basicData.password) {
                    let value = `${basicData.username}:${basicData.password}`;
                    addHeader({ header: "Authorization", value: `Basic ${btoa(value)}`, generated: true }, headers);
                } else {
                    removeHeader("Authorization", headers);
                }
                break;
            case AuthType.Bearer:
                let bearerData = auth.data as BearerAuthData;
                if (bearerData && bearerData.token) {
                    addHeader({ header: "Authorization", value: `Bearer ${bearerData.token}`, generated: true }, headers);
                } else {
                    removeHeader("Authorization", headers);
                }
                break;
            default:
        }

        updateHeaders(headers);
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

    const updateHeaders = (headers: HttpHeader[]) => {
        let request = { ...currentFile.request ?? {} as Request };
        request.headers = headers;
        updateRequest(request);
        setRequestTabCount("tab-headers", headers.length);
    };

    return (
        <div id="tab-auth-panel" class="p-3 pt-0">
            <div>
                <label for="type-select" class="text-xs text-neutral-400 mr-3">Auth Type</label>
                <select id="type-select" class="h-7 w-28 p-0 px-2 mb-4 rounded-[3px] bg-transparent text-xs text-neutral-300 border-neutral-600 focus:ring-0 focus:border-neutral-400"
                    onChange={(e) => onTypeChange(+e.currentTarget.value)} value={auth().type}>
                    <option selected value={AuthType.None}>None</option>
                    <option value={AuthType.Basic}>Basic Auth</option>
                    <option value={AuthType.Bearer}>Bearer Token</option>
                </select>
            </div>

            <Show when={auth().type === AuthType.Basic}>
                <div class="inline-grid grid-cols-3 gap-y-2 w-72">
                    <div class="flex items-center">
                        <label for="username-input" class="text-sm text-neutral-400">Username</label>
                    </div>
                    <input id="username-input"
                        spellcheck={false}
                        autocorrect="off"
                        placeholder="username"
                        value={(auth().data as BasicAuthData)?.username ?? ""}
                        class="col-span-2 h-7 w-36 p-1 px-2 rounded-[3px] bg-transparent text-sm text-neutral-300 border-neutral-600 focus:ring-0 focus:border-neutral-400 placeholder:text-neutral-500"
                        onChange={(e) => onBasicAuthFieldChange("username", e.currentTarget.value)} />
                    <div class="flex items-center">
                        <label for="password-input" class="text-sm text-neutral-400">Password</label>
                    </div>
                    <input id="password-input" type="password" placeholder="password" value={(auth().data as BasicAuthData)?.password ?? ""} // TODO: mask value
                        class="col-span-2 h-7 w-36 p-1 px-2 rounded-[3px] bg-transparent text-sm text-neutral-300 border-neutral-600 focus:ring-0 focus:border-neutral-400 placeholder:text-neutral-500"
                        onChange={(e) => onBasicAuthFieldChange("password", e.currentTarget.value)} />
                </div>
            </Show>

            <Show when={auth().type === AuthType.Bearer}>
                <div class="inline-grid grid-cols-3 gap-y-2 w-84">
                    <div class="flex items-center">
                        <label for="token-input" class="text-sm text-neutral-400">Token</label>
                    </div>
                    <input id="token-input"
                        spellcheck={false}
                        autocorrect="off"
                        placeholder="token"
                        value={(auth().data as BearerAuthData)?.token ?? ""}
                        class="col-span-2 h-7 p-0 px-2 rounded-[3px] bg-transparent text-sm text-neutral-300 border-neutral-600 focus:ring-0 focus:border-neutral-400 placeholder:text-neutral-500"
                        onChange={(e) => onBearerAuthTokenChange(e.currentTarget.value)} />
                </div>
            </Show>
        </div>
    );
}
