import { atom } from 'nanostores';
import { invoke } from "@tauri-apps/api/core";
import { setRequestTabActive, setRequestTabCount, setResponseTabActive, setResponseTabCount } from './layoutState';
import { AuthType, parseAuthData, type AuthState } from './authState';
import { addSavedFile, selectEntity, selectEntityPath, type FolderEntity } from './folderState';
import { v4 as uuidv4 } from 'uuid';

export type TestFile = {
    name?: string,
    id?: string,
    platformId?: string,
    description?: string,
    disabled?: boolean,
    project?: string,
    env?: string,
    tags?: string,
    requires?: string,
    iterate?: number,

    request?: Request,
    compare?: Compare,
    response?: Response,
    setup?: Setup,
    stages?: Stage[],
    cleanup?: Cleanup,
    variables?: Variable[],
};

export type Request = {
    method?: HttpVerb,
    url?: string,
    params?: HttpParameter[],
    headers?: HttpHeader[],
    body?: Object,
};

export type Compare = {
    method?: HttpVerb,
    url?: string,
    params?: HttpParameter[],
    addParams?: HttpParameter[],
    ignoreParams?: string[],
    headers?: HttpHeader[],
    addHeaders?: HttpHeader[],
    ignoreHeaders?: string[],
    body?: Object,
    bodySchema?: Object,
    strict?: boolean,
};

export type Response = {
    status?: number,
    time?: number,
    headers?: HttpHeader[],
    ignore?: string[],
    extract?: Extract[],
    body?: Object,
    bodySchema?: Object,
    strict?: boolean,
};

export type Setup = {
    request?: Request,
    response?: Response,
};

export type Stage = {
    name?: string,
    request?: Request,
    compare?: Compare,
    response?: Response,
    delay?: number,
};

export type Cleanup = {
    onsuccess?: Request,
    onfailure?: Request,
    always?: Request,
};

export type Variable = {
    name?: string,
    type?: VariableType,
    value?: any,
    valueSet?: any[],
    file?: string,
    min?: number,
    max?: number,
    oneOf?: any[],
    noneOf?: any[],
    length?: number,
    minLength?: number,
    maxLength?: number,
    schema?: Object,
};

export enum HttpVerb {
    GET = "Get",
    POST = "Post",
    PUT = "Put",
    PATCH = "Patch",
    DELETE = "Delete",
};

export type HttpParameter = {
    param: string,
    value: string,
    generated: boolean,
};

export type HttpHeader = {
    header: string,
    value: string,
    generated: boolean,
};

export type Extract = {
    name?: string,
    field?: string,
};

export enum VariableType {
    INTEGER = "Int",
    STRING = "String",
    DATE = "Date",
    FLOAT = "Float",
    OBJECT = "Object",
    LIST = "List",
    BOOLEAN = "Boolean",
    EMAIL = "Email",
    NAME = "Name",
    DATETIME = "Datetime",
};

export type VariableModifier = {
    type?: VariableModifierType,
    value?: number,
    unit?: VariableModifierUnit,
};

export enum VariableModifierType {
    ADD = "add",
    SUBTRACT = "subtract",
};

export enum VariableModifierUnit {
    DAYS = "days",
    WEEKS = "weeks",
    MONTHS = "months",
};

export type File = {
    name: string,
    path: string,
};

export type HttpResponse = {
    status: number,
    time?: number,
    size?: number,
    headers: HttpHeader[],
    body?: string,
};

export type FileState = {
    id: string,
    file?: File,
    testFile: TestFile,
    response?: HttpResponse,
    auth: AuthState,
};

export type EditorState = {
    currentFile: number,
    files: FileState[],
};

const getNewFile = () => {
    return {
        id: uuidv4(),
        file: undefined,
        testFile: {
            request: {
                method: HttpVerb.GET,
            },
        },
        response: undefined,
        auth: { type: AuthType.None },
    };
}

const initState: EditorState = {
    currentFile: 0,
    files: [getNewFile()]
};

export const $editorState = atom(initState);

export const updateFile = (file: TestFile) => {
    console.log("editor state - updating file: ", file);
    let currentState = $editorState.get();
    let currentFile = currentState.files[currentState.currentFile];
    currentFile.testFile = { ...file };
    $editorState.set({ ...currentState });
    resetTabCounts(currentFile);
}

export const updateRequest = (request: Request) => {
    console.log("editor state - updating request: ", request.params);
    let currentState = $editorState.get();
    currentState.files[currentState.currentFile].testFile.request = { ...request };
    $editorState.set({ ...currentState });
};

export const updateAuth = (auth: AuthState) => {
    console.log("editor state - updating auth: ", auth);
    let currentState = $editorState.get();
    currentState.files[currentState.currentFile].auth = { ...auth };
    $editorState.set({ ...currentState });
    setRequestTabCount("tab-auth", auth.type === AuthType.None ? 0 : 1);
};

export const selectFile = (index: number) => {
    console.log("selecting test file at index ", index);
    let state = $editorState.get();
    state.currentFile = index;
    $editorState.set({ ...state });
    resetTabs(state.files[state.currentFile]);
    selectEntityPath(state.files[state.currentFile].file?.path);
};

export const addNewFile = () => {
    let state = $editorState.get()
    let file = getNewFile();
    state.currentFile++;
    state.files.push(file);

    $editorState.set({ ...state });
    resetTabs(file);
    selectEntity(-1);
};

export const openFile = async (entity: FolderEntity) => {
    if (entity.isDirectory) return;
    console.log("opening file at path ", entity.path);

    let currentState = $editorState.get();
    let foundIndex = currentState.files.findIndex((f) => f.file?.path === entity.path);
    if (foundIndex > -1) {
        console.log("file is already open, selecting tab at index ", foundIndex);
        selectFile(foundIndex);
        return;
    }

    let file: File = { name: entity.name, path: entity.path };
    let testFile: TestFile = await invoke("open_file", { file: file });
    console.log("file contents: ", testFile);

    // add content-type header, if applicable
    if (testFile.request?.body && !(testFile.request?.headers ?? []).some(h => h.header.toLowerCase() === "content-type")) {
        if (!testFile.request) testFile.request = {};
        if (!testFile.request.headers) testFile.request.headers = [];
        testFile.request.headers.push({ header: "Content-Type", value: "application/json", generated: true });
    }

    // add auth data, if applicable
    let auth = parseAuthData(testFile.request?.headers);

    let fileState = { id: uuidv4(), file: file, testFile: testFile, auth: auth }
    currentState.files.push(fileState);
    currentState.currentFile++;

    $editorState.set({ ...currentState });
    resetTabs(fileState);
    selectEntityPath(file.path);
};

export const saveFile = async () => {
    let state = $editorState.get();
    console.log("saving file at index ", state.currentFile);
    let file = state.files[state.currentFile];
    let modifiedFile = pruneGeneratedValues(file);
    let savedFile: File | undefined;

    if (modifiedFile.file) {
        console.log("saving existing file");
        savedFile = await invoke("save_existing_file", { file: modifiedFile.file!, testFile: modifiedFile.testFile });
        if (savedFile) {
            console.log("successfully saved file");
        } else {
            console.log("failed to save file");
        }
        return;
    }

    console.log("saving new file");
    savedFile = await invoke("save_new_file", { testFile: modifiedFile.testFile });
    if (!savedFile) {
        console.log("failed to save file");
        return;
    }

    file.file = savedFile;
    $editorState.set({ ...state });
    await addSavedFile(savedFile!);
    console.log("successfully saved file");
};

export const closeFile = (index: number) => {
    console.log("closing file at index ", index);
    let state = $editorState.get();
    state.files.splice(index, 1);

    // if there are no files left, open a new scratch pad
    if (state.files.length === 0) {
        state.currentFile = 0;
        console.log("new current file: 0");
        state.files.push(getNewFile());
        resetTabs(state.files[0]);
        selectEntity(-1);
    } else if (index <= state.currentFile) {
        // adjust the selected index, if applicable
        state.currentFile = Math.max(state.currentFile - 1, 0);
        console.log("new current file: ", state.currentFile);
        resetTabs(state.files[state.currentFile]);
        selectEntityPath(state.files[state.currentFile].file?.path);
    }

    $editorState.set({ ...state });
};

export const makeRequest = async () => {
    let state = $editorState.get();
    let file = state.files[state.currentFile];
    console.log("making http request: ", file.testFile.request);
    if (!file.testFile.request?.url) {
        console.log("no request url");
        return;
    }

    let response: HttpResponse = await invoke("make_request", { testFile: file.testFile });
    console.log("http response: ", response);
    let size = response.headers.find(h => h.header.toLowerCase() === "content-length")?.value;
    response.size = size ? +size : undefined;

    file.response = response;
    $editorState.set({ ...state });
    setResponseTabCount("tab-body", response.body ? 1 : 0);
    setResponseTabCount("tab-headers", response.headers.length);
};

export const saveResponseBody = async () => {
    let state = $editorState.get();
    console.log("saving response body from file at index ", state.currentFile);
    let body = state.files[state.currentFile].response?.body;
    if (!body) {
        console.log("no response body found");
        return;
    }

    let prettifiedBody = JSON.stringify(JSON.parse(body), undefined, 2);
    let savedFile: File | undefined;
    savedFile = await invoke("save_response_body", { body: prettifiedBody });
    if (!savedFile) {
        console.log("failed to save response body");
        return;
    }

    console.log("successfully saved response body at path ", savedFile.path);
};

const resetTabCounts = (file: FileState) => {
    console.log(file);
    setRequestTabCount("tab-params", file.testFile.request?.params?.length ?? 0);
    setRequestTabCount("tab-headers", file.testFile.request?.headers?.length ?? 0);
    setRequestTabCount("tab-auth", (file.auth ?? {}).type !== AuthType.None ? 1 : 0);
    setRequestTabCount("tab-body", file.testFile.request?.body ? 1 : 0);
    setResponseTabCount("tab-body", file.response?.body ? 1 : 0);
    setResponseTabCount("tab-headers", file.response?.headers?.length ?? 0);
};

const resetTabs = (file: FileState) => {
    resetTabCounts(file);
    setRequestTabActive(1, false);
    setResponseTabActive(1, false);
}

const pruneGeneratedValues = (file: FileState) => {
    let headers = file.testFile.request?.headers;
    let params = file.testFile.request?.params;
    if (!headers && !params) return file;

    let filteredHeaders = headers?.filter(h => !h.generated) ?? [];
    let filteredParams = params?.filter(p => !p.generated) ?? [];
    if (filteredHeaders.length === (headers?.length ?? 0) && filteredParams.length === (params?.length ?? 0)) return file;

    let modifiedFile = JSON.parse(JSON.stringify(file)); // deep copy
    modifiedFile.testFile.request!.headers = filteredHeaders.length > 0 ? filteredHeaders : undefined;
    modifiedFile.testFile.request!.params = filteredParams.length > 0 ? filteredParams : undefined;
    return modifiedFile;
};
