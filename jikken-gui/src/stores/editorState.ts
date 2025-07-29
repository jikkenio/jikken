import { atom } from "nanostores";
import { invoke } from "@tauri-apps/api/core";
import { setRequestTabActive, setRequestTabCount, setResponseTabActive, setResponseTabCount } from './layoutState';
import { parseAuthData, type AuthState } from './authState';
import { addSavedFile, loadFolder, selectEntity, selectEntityPath } from './folderState';
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

export type RawConfigFile = {
  settings?: ConfigSettings;
  globals?: Object,
}

export type ConfigFile = {
  settings?: ConfigSettings;
  globals?: Map<string, string>,
};

export type ConfigSettings = {
  apiKey?: string,
  bypassCertVerification?: boolean,
  continueOnFailure?: boolean,
  devMode?: boolean,
  environment?: string,
}

export type File = {
  name: string;
  path: string;
};

export type HttpResponse = {
  status: number;
  time: number;
  size: number;
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

export const openFile = async (type: EntityType, name: string, path: string) => {
  if (type === EntityType.Directory) return;
  console.log("editor state - opening file at path ", path);

  let currentState = $editorState.get();
  let foundIndex = currentState.files.findIndex(
    (f) => f.file?.path === path
  );
  if (foundIndex > -1) {
    console.log("file is already open, selecting tab at index ", foundIndex);
    selectFile(foundIndex);
    return;
  }

  let file: File = { name: name, path: path };
  if (type === EntityType.Test) {
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
  currentState.currentFile = currentState.files.length - 1;

  $editorState.set({ ...currentState });
  resetTabs(testFileState);
};

const openConfigFile = async (file: File) => {
  let rawFile: RawConfigFile = await invoke("open_config_file", { file: file });
  console.log("config file contents: ", rawFile);

  let configFile: ConfigFile = { settings: rawFile.settings };
  if (rawFile.globals) {
    configFile.globals = new Map(Object.entries(rawFile.globals!));
  }

  let currentState = $editorState.get();
  let newIndex = currentState.configFiles.length;
  let fileState = { id: uuidv4(), file: file, type: EntityType.Config, index: newIndex };
  currentState.files.push(fileState);
  currentState.configFiles.push(configFile);
  currentState.currentFile++;

  $editorState.set({ ...currentState });
  resetTabs(undefined);
};

export const saveCurrentFile = async () => {
  let state = $editorState.get();
  let file = state.files[state.currentFile];
  let savedFile: File | undefined;

  if (file.type === EntityType.Test) {
    console.log("editor state - saving test file at index ", state.currentFile);
    let testFile = state.testFiles[file.index];
    let modifiedFile: TestFileState = pruneGeneratedValues(testFile);
    savedFile = await saveTestFile(file.file, modifiedFile.testFile);
    console.log("successfully saved test file");
  } else {
    console.log("editor state - saving config file at index ", state.currentFile);
    let configFile = state.configFiles[file.index];
    savedFile = await saveConfigFile(file.file, configFile);
  }

  file.file = savedFile;
  $editorState.set({ ...state });
  await addSavedFile(savedFile!);
};

export const createNewFile = async () => {
  console.log("creating new test file");
  let testFile: TestFile = getNewTestFile().testFile;
  testFile.request!.url = "https://api.jikken.io";
  let savedFile = await saveTestFile(undefined, testFile);
  if (!savedFile) {
    console.log("failed to save new test file");
    return;
  }

  let folderPath = savedFile!.path.replace(savedFile.name, "");
  await loadFolder(folderPath);
  await openFile(EntityType.Test, savedFile.name, savedFile.path);
  console.log("successfully created new test file at path ", savedFile?.path);
};

export const saveTestFile = async (file: File | undefined, testFile: TestFile) => {
  let savedFile: File | undefined;
  if (file) {
    console.log("saving existing test file");
    savedFile = await invoke("save_existing_test_file", { file: file!, testFile: testFile });
    if (savedFile) {
      console.log("successfully saved test file");
    } else {
      console.log("failed to save test file");
    }
    return savedFile;
  }

  console.log("saving new test file");
  savedFile = await invoke("save_new_test_file", {
    testFile: testFile,
  });
  if (savedFile) {
    console.log("successfully saved new test file");
  } else {
    console.log("failed to save new test file");
  }
  return savedFile;
};

export const saveConfigFile = async (file: File | undefined, configFile: ConfigFile) => {
  let savedFile: File | undefined;
  if (file) {
    console.log("saving existing config file");
    savedFile = await invoke("save_existing_config_file", { file: file!, configFile: configFile });
    if (savedFile) {
      console.log("successfully saved config file");
    } else {
      console.log("failed to save config file");
    }
    return savedFile;
  }

  console.log("saving new config file");
  savedFile = await invoke("save_new_config_file", {
    configFile: configFile,
  });
  if (savedFile) {
    console.log("successfully saved new config file");
  } else {
    console.log("failed to save new config file");
  }
  return savedFile;
};

export const closeFile = (index: number) => {
  console.log("editor state - closing file at index ", index);
  let state = $editorState.get();
  let fileType = state.files[index].type;
  let fileIndex = state.files[index].index;
  state.files.splice(index, 1);
  state.files.filter((f) => f.type === fileType && f.index > fileIndex).forEach((f) => f.index--);
  state.files = JSON.parse(JSON.stringify(state.files));

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

  console.log("making http request: ", testFile.testFile.request);
  if (!testFile.testFile.request?.url) {
    console.log("no request url");
    return;
  }

  testFile.response = undefined;
  testFile.executing = true;
  $editorState.set({ ...state });
  clearNotification();
  let response: HttpResponse;

  try {
    response = await invoke("make_request", { testFile: testFile.testFile });
  } catch (ex) {
    triggerBanner(NotificationType.Error, "Failed to execute HTTP request");
    console.log("Failed to make network request: ", ex);
    testFile.executing = false;
    $editorState.set({ ...state });
    return;
  }

  if (!response) {
    triggerBanner(NotificationType.Error, "Failed to execute HTTP request");
    console.log("Failed to make network request, null response");
    testFile.executing = false;
    $editorState.set({ ...state });
    return;
  }

  console.log("http response: ", response);
  testFile.response = response;
  testFile.executing = false;
  $editorState.set({ ...state });
  setResponseTabCount("tab-body", response.body ? 1 : 0);
  setResponseTabCount("tab-headers", response.headers.length);

  // Initialize Split.js now that response is available and panels should be visible
  setTimeout(() => {
    if (typeof window !== 'undefined' && (window as any).initializeRequestResponseSplit) {
      (window as any).initializeRequestResponseSplit();
    }
  }, 100);
};

// Pretty printing functions for save functionality
const prettyPrintHtml = (html: string): string => {
  try {
    const parser = new DOMParser();
    const doc = parser.parseFromString(html, 'text/html');

    const errorNode = doc.querySelector('parsererror');
    if (errorNode) {
      return html;
    }

    return formatElement(doc.documentElement, 0);
  } catch {
    return html;
  }
};

const prettyPrintXml = (xml: string): string => {
  try {
    const parser = new DOMParser();
    const doc = parser.parseFromString(xml, 'application/xml');

    const errorNode = doc.querySelector('parsererror');
    if (errorNode) {
      return xml;
    }

    return formatElement(doc.documentElement, 0);
  } catch {
    return xml;
  }
};

const formatElement = (element: Element, depth: number): string => {
  const indent = '  '.repeat(depth);

  let result = `${indent}<${element.tagName.toLowerCase()}`;

  // Add attributes
  for (let i = 0; i < element.attributes.length; i++) {
    const attr = element.attributes[i];
    result += ` ${attr.name}="${attr.value}"`;
  }

  if (element.children.length === 0 && !element.textContent?.trim()) {
    result += ' />';
    return result;
  }

  result += '>';

  const textContent = element.textContent?.trim();
  const hasElementChildren = element.children.length > 0;

  if (hasElementChildren) {
    result += '\n';
    for (let i = 0; i < element.children.length; i++) {
      result += formatElement(element.children[i], depth + 1);
      if (i < element.children.length - 1) {
        result += '\n';
      }
    }
    result += `\n${indent}`;
  } else if (textContent) {
    result += textContent;
  }

  result += `</${element.tagName.toLowerCase()}>`;
  return result;
};

export const saveResponseBody = async () => {
  let state = $editorState.get();
  console.log("saving response body from file at index ", state.currentFile);
  let index = state.files[state.currentFile].index;
  let response = state.testFiles[index].response;
  if (!response?.body) {
    console.log("no response body found");
    return;
  }

  // Check content type to determine formatting
  let contentType: string | undefined;
  if (response.headers) {
    const contentTypeHeader = response.headers.find(h => h.header.toLowerCase() === 'content-type');
    contentType = contentTypeHeader?.value?.toLowerCase();
  }

  let bodyToSave = response.body;

  // Format based on content type
  if (contentType?.includes('json') || (!contentType && response.body.trim().startsWith('{'))) {
    try {
      bodyToSave = JSON.stringify(JSON.parse(response.body), undefined, 2);
    } catch {
      console.log("failed to parse as JSON, saving raw body");
    }
  } else if (contentType?.includes('html')) {
    bodyToSave = prettyPrintHtml(response.body);
  } else if (contentType?.includes('xml')) {
    bodyToSave = prettyPrintXml(response.body);
  }

  let savedFile: File | undefined;
  savedFile = await invoke("save_response_body", { body: bodyToSave });
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
