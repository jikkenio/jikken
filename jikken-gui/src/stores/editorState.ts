import { atom } from "nanostores";
import { invoke } from "@tauri-apps/api/core";
import { setRequestTabActive, setRequestTabCount, setResponseTabActive, setResponseTabCount } from './layoutState';
import { parseAuthData, type AuthState } from './authState';
import { addSavedFile, selectEntity, selectEntityPath, type FolderEntity } from './folderState';
import { v4 as uuidv4 } from 'uuid';
import { clearNotification, NotificationType, triggerBanner } from './notificationState';
import { AuthType, EntityType, HttpVerb } from './enum';

export type TestFile = {
  name?: string;
  id?: string;
  platformId?: string;
  description?: string;
  disabled?: boolean;
  project?: string;
  env?: string;
  tags?: string;
  requires?: string;
  iterate?: number;

  request?: Request;
  compare?: Compare;
  response?: Response;
  setup?: Setup;
  stages?: Stage[];
  cleanup?: Cleanup;
  variables?: Variable[];
};

export type Request = {
  method?: HttpVerb;
  url?: string;
  params?: HttpParameter[];
  headers?: HttpHeader[];
  body?: Object;
};

export type Compare = {
  method?: HttpVerb;
  url?: string;
  params?: HttpParameter[];
  addParams?: HttpParameter[];
  ignoreParams?: string[];
  headers?: HttpHeader[];
  addHeaders?: HttpHeader[];
  ignoreHeaders?: string[];
  body?: Object;
  bodySchema?: Object;
  strict?: boolean;
};

export type Response = {
  status?: number;
  time?: number;
  headers?: HttpHeader[];
  ignore?: string[];
  extract?: Extract[];
  body?: Object;
  bodySchema?: Object;
  strict?: boolean;
};

export type Setup = {
  request?: Request;
  response?: Response;
};

export type Stage = {
  name?: string;
  request?: Request;
  compare?: Compare;
  response?: Response;
  delay?: number;
};

export type Cleanup = {
  onsuccess?: Request;
  onfailure?: Request;
  always?: Request;
};

export type Variable = {
  name?: string;
  type?: VariableType;
  value?: any;
  valueSet?: any[];
  file?: string;
  min?: number;
  max?: number;
  oneOf?: any[];
  noneOf?: any[];
  length?: number;
  minLength?: number;
  maxLength?: number;
  schema?: Object;
};

export type HttpParameter = {
  param: string;
  value: string;
  generated: boolean;
};

export type HttpHeader = {
  header: string;
  value: string;
  generated: boolean;
};

export type Extract = {
  name?: string;
  field?: string;
};

enum VariableType {
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
  type?: VariableModifierType;
  value?: number;
  unit?: VariableModifierUnit;
};

enum VariableModifierType {
  ADD = "add",
  SUBTRACT = "subtract",
};

enum VariableModifierUnit {
  DAYS = "days",
  WEEKS = "weeks",
  MONTHS = "months",
};

export type ConfigFile = {
  apiKey?: string,
  bypassCertVerification: boolean,
  continueOnFailure: boolean,
  devMode?: boolean,
  environment?: string,
  globals: GlobalVariable[],
};

export type GlobalVariable = {
  key?: string,
  value?: string,
};

export type File = {
  name: string;
  path: string;
};

export type HttpResponse = {
  status: number;
  time?: number;
  size?: number;
  headers: HttpHeader[];
  body?: string;
};

export type FileState = {
  id: string,
  file?: File,
  type: EntityType,
  index: number,
};

export type TestFileState = {
  testFile: TestFile,
  executing: boolean,
  response?: HttpResponse,
  auth: AuthState,
}

export type EditorState = {
  currentFile: number,
  files: FileState[],
  testFiles: TestFileState[],
  configFiles: ConfigFile[],
};

const getNewFileState = (index: number) => {
  return {
    id: uuidv4(),
    file: undefined,
    type: EntityType.Test,
    index: index,
  };
}

const getNewTestFile = () => {
  return {
    testFile: {
      request: {
        method: HttpVerb.GET,
      },
    },
    executing: false,
    response: undefined,
    auth: { type: AuthType.None },
  };
}

const initState: EditorState = {
  currentFile: 0,
  files: [getNewFileState(0)],
  testFiles: [getNewTestFile()],
  configFiles: [],
};

export const $editorState = atom(initState);

export const updateTestFile = (file: TestFile) => {
  console.log("editor state - updating test file: ", file);
  let currentState = $editorState.get();
  let index = currentState.files[currentState.currentFile].index;
  let currentFile = currentState.testFiles[index];
  currentFile.testFile = { ...file };
  $editorState.set({ ...currentState });
  resetTabCounts(currentFile);
}

export const updateRequest = (request: Request) => {
  console.log("editor state - updating request: ", request);
  let currentState = $editorState.get();
  let index = currentState.files[currentState.currentFile].index;
  let currentFile = currentState.testFiles[index];
  currentFile.testFile.request = { ...request };
  $editorState.set({ ...currentState });
};

export const updateAuth = (auth: AuthState) => {
  console.log("editor state - updating auth: ", auth);
  let currentState = $editorState.get();
  let index = currentState.files[currentState.currentFile].index;
  let currentFile = currentState.testFiles[index];
  currentFile.auth = { ...auth };
  $editorState.set({ ...currentState });
  setRequestTabCount("tab-auth", auth.type === AuthType.None ? 0 : 1);
};

export const updateConfigFile = (file: ConfigFile) => {
  console.log("editor state - updating config file: ", file);
  let currentState = $editorState.get();
  let index = currentState.files[currentState.currentFile].index;
  currentState.configFiles[index] = file;
  $editorState.set({ ...currentState });
};

export const selectFile = (index: number) => {
  console.log("editor state - selecting file at index ", index);
  let state = $editorState.get();
  state.currentFile = index;
  let file = state.files[index];
  $editorState.set({ ...state });
  resetTabs(file.type === EntityType.Test ? state.testFiles[file.index] : undefined);
  selectEntityPath(state.files[state.currentFile].file?.path);
};

export const addNewFile = () => {
  console.log("editor state - adding new scratch pad");
  let state = $editorState.get();

  let newIndex = state.testFiles.length;
  let newFile = getNewFileState(newIndex);
  let newTestFile = getNewTestFile();
  state.files.push(newFile);
  state.testFiles.push(newTestFile);

  state.currentFile = state.files.length - 1;
  $editorState.set({ ...state });
  resetTabs(newTestFile);
  selectEntity(-1);
};

export const openFile = async (entity: FolderEntity) => {
  if (entity.type === EntityType.Directory) return;
  console.log("editor state - opening file at path ", entity.path);

  let currentState = $editorState.get();
  let foundIndex = currentState.files.findIndex(
    (f) => f.file?.path === entity.path
  );
  if (foundIndex > -1) {
    console.log("file is already open, selecting tab at index ", foundIndex);
    selectFile(foundIndex);
    return;
  }

  let file: File = { name: entity.name, path: entity.path };
  if (entity.type === EntityType.Test) {
    openTestFile(file);
  } else {
    openConfigFile(file);
  }

  selectEntityPath(file.path);
};

const openTestFile = async (file: File) => {
  let testFile: TestFile = await invoke("open_test_file", { file: file });
  console.log("test file contents: ", testFile);

  // add content-type header, if applicable
  if (
    testFile.request?.body &&
    !(testFile.request?.headers ?? []).some(
      (h) => h.header.toLowerCase() === "content-type"
    )
  ) {
    if (!testFile.request) testFile.request = {};
    if (!testFile.request.headers) testFile.request.headers = [];
    testFile.request.headers.push({
      header: "Content-Type",
      value: "application/json",
      generated: true,
    });
  }

  // add auth data, if applicable
  let auth = parseAuthData(testFile.request?.headers);

  let currentState = $editorState.get();
  let newIndex = currentState.testFiles.length;
  let fileState = { id: uuidv4(), file: file, type: EntityType.Test, index: newIndex };
  let testFileState = { testFile: testFile, executing: false, auth: auth };
  currentState.files.push(fileState);
  currentState.testFiles.push(testFileState);
  currentState.currentFile++;

  $editorState.set({ ...currentState });
  resetTabs(testFileState);
};

const openConfigFile = async (file: File) => {
  let configFile: ConfigFile = await invoke("open_config_file", { file: file });
  console.log("config file contents: ", configFile);

  let currentState = $editorState.get();
  let newIndex = currentState.configFiles.length;
  let fileState = { id: uuidv4(), file: file, type: EntityType.Config, index: newIndex };
  currentState.files.push(fileState);
  currentState.configFiles.push(configFile);
  currentState.currentFile++;

  $editorState.set({ ...currentState });
  resetTabs(undefined);
};

export const saveFile = async () => {
  let state = $editorState.get();
  if (state.files[state.currentFile].type === EntityType.Test) {
    saveTestFile(state);
  } else {
    saveConfigFile(state);
  }
};

export const saveTestFile = async (state: EditorState) => {
  console.log("editor state - saving test file at index ", state.currentFile);

  let file = state.files[state.currentFile];
  let testFile = state.testFiles[file.index];
  let modifiedFile: TestFileState = pruneGeneratedValues(testFile);
  let savedFile: File | undefined;

  if (file.file) {
    console.log("saving existing test file");
    savedFile = await invoke("save_existing_test_file", { file: file.file!, testFile: modifiedFile.testFile });
    if (savedFile) {
      console.log("successfully saved test file");
    } else {
      console.log("failed to save test file");
    }
    return;
  }

  console.log("saving new test file");
  savedFile = await invoke("save_new_test_file", {
    testFile: modifiedFile.testFile,
  });
  if (!savedFile) {
    console.log("failed to save test file");
    return;
  }

  file.file = savedFile;
  $editorState.set({ ...state });
  await addSavedFile(savedFile!);
  console.log("successfully saved test file");
};

export const saveConfigFile = async (state: EditorState) => {
  console.log("editor state - saving config file at index ", state.currentFile);

  let file = state.files[state.currentFile];
  let configFile = state.configFiles[file.index];
  let savedFile: File | undefined;

  if (file.file) {
    console.log("saving existing config file");
    savedFile = await invoke("save_existing_config_file", { file: file.file!, configFile: configFile });
    if (savedFile) {
      console.log("successfully saved config file");
    } else {
      console.log("failed to save config file");
    }
    return;
  }

  console.log("saving new config file");
  savedFile = await invoke("save_new_config_file", {
    configFile: configFile,
  });
  if (!savedFile) {
    console.log("failed to save config file");
    return;
  }

  file.file = savedFile;
  $editorState.set({ ...state });
  await addSavedFile(savedFile!);
  console.log("successfully saved config file");
};

export const closeFile = (index: number) => {
  console.log("editor state - closing file at index ", index);
  let state = $editorState.get();
  let fileType = state.files[index].type;
  let fileIndex = state.files[index].index;
  state.files.splice(index, 1);

  if (fileType === EntityType.Test) {
    state.testFiles.splice(fileIndex, 1);
  } else {
    state.configFiles.splice(fileIndex, 1);
  }

  // if there are no files left, open a new scratch pad
  if (state.files.length === 0) {
    state.currentFile = 0;
    console.log("new current file: 0");
    state.files.push(getNewFileState(0));
    state.testFiles.push(getNewTestFile());
    resetTabs(state.testFiles[0]);
    selectEntity(-1);
  } else if (index <= state.currentFile) {
    // adjust the selected index, if applicable
    state.currentFile = Math.max(state.currentFile - 1, 0);
    console.log("new current file: ", state.currentFile);
    let file = state.files[state.currentFile];
    resetTabs(file.type === EntityType.Test ? state.testFiles[file.index] : undefined);
    selectEntityPath(state.files[state.currentFile].file?.path);
  }

  $editorState.set({ ...state });
};

export const makeRequest = async () => {
  let state = $editorState.get();
  let file = state.files[state.currentFile];
  let testFile = state.testFiles[file.index];
  testFile.response = undefined;
  testFile.executing = true;
  $editorState.set({ ...state });

  console.log("making http request: ", testFile.testFile.request);
  if (!testFile.testFile.request?.url) {
    console.log("no request url");
    return;
  }
  let response: HttpResponse;

  clearNotification();

  try {
    response = await invoke("make_request", { testFile: testFile.testFile });
  } catch (ex) {
    triggerBanner(NotificationType.Error, "Failed to execute HTTP request");
    console.log("Failed to make network request: ", ex);
    return;
  }

  if (!response) {
    triggerBanner(NotificationType.Error, "Failed to execute HTTP request");
    console.log("Failed to make network request, null response");
    return;
  }

  console.log("http response: ", response);
  let size = response.headers.find(
    (h) => h.header.toLowerCase() === "content-length"
  )?.value;
  response.size = size ? +size : undefined;

  testFile.response = response;
  testFile.executing = false;
  $editorState.set({ ...state });
  setResponseTabCount("tab-body", response.body ? 1 : 0);
  setResponseTabCount("tab-headers", response.headers.length);
};

export const saveResponseBody = async () => {
  let state = $editorState.get();
  console.log("saving response body from file at index ", state.currentFile);
  let index = state.files[state.currentFile].index;
  let body = state.testFiles[index].response?.body;
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

const resetTabCounts = (file: TestFileState | undefined) => {
  setRequestTabCount("tab-params", file?.testFile.request?.params?.length ?? 0);
  setRequestTabCount("tab-headers", file?.testFile.request?.headers?.length ?? 0);
  setRequestTabCount("tab-auth", (file?.auth ?? {}).type !== AuthType.None ? 1 : 0);
  setRequestTabCount("tab-body", file?.testFile.request?.body ? 1 : 0);
  setResponseTabCount("tab-body", file?.response?.body ? 1 : 0);
  setResponseTabCount("tab-headers", file?.response?.headers?.length ?? 0);
};

const resetTabs = (testFile: TestFileState | undefined) => {
  resetTabCounts(testFile);
  setRequestTabActive(1, false);
  setResponseTabActive(1, false);
}

const pruneGeneratedValues = (file: TestFileState) => {
  let headers = file.testFile.request?.headers;
  let params = file.testFile.request?.params;
  if (!headers && !params) return file;

  let filteredHeaders = headers?.filter((h) => !h.generated) ?? [];
  let filteredParams = params?.filter((p) => !p.generated) ?? [];
  if (
    filteredHeaders.length === (headers?.length ?? 0) &&
    filteredParams.length === (params?.length ?? 0)
  )
    return file;

  let modifiedFile = JSON.parse(JSON.stringify(file)); // deep copy
  modifiedFile.testFile.request!.headers =
    filteredHeaders.length > 0 ? filteredHeaders : undefined;
  modifiedFile.testFile.request!.params =
    filteredParams.length > 0 ? filteredParams : undefined;
  return modifiedFile;
};
