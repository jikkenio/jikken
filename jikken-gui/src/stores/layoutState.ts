import { map } from "nanostores";
import { ViewMode } from "./enum";

export type RequestResponseTab = {
    label: string,
    id: string,
    items: number,
    index: number,
    showCount: boolean,
};

export type LayoutState = {
    viewMode: ViewMode,
    folderPanelVisible: boolean,
    requestTabPanelVisible: boolean,
    responseTabPanelVisible: boolean,
    requestTabs: RequestResponseTab[],
    responseTabs: RequestResponseTab[],
    requestTabIndex: number,
    responseTabIndex: number,
};

const initState: LayoutState = {
    viewMode: ViewMode.API,
    folderPanelVisible: true,
    requestTabPanelVisible: true,
    responseTabPanelVisible: true,
    requestTabs: [{
        label: "Params",
        id: "tab-params",
        items: 0,
        index: 1,
        showCount: true,
    },
    {
        label: "Headers",
        id: "tab-headers",
        items: 0,
        index: 2,
        showCount: true,
    },
    {
        label: "Auth",
        id: "tab-auth",
        items: 0,
        index: 3,
        showCount: false,
    },
    {
        label: "Body",
        id: "tab-body",
        items: 0,
        index: 4,
        showCount: false
    },],
    responseTabs: [{
        label: "Body",
        id: "tab-body",
        items: 0,
        index: 1,
        showCount: false,
    },
    {
        label: "Headers",
        id: "tab-headers",
        items: 0,
        index: 2,
        showCount: true,
    },],
    requestTabIndex: 1,
    responseTabIndex: 1,
};

const requestTabIndexByName = new Map([["tab-params", 1], ["tab-headers", 2], ["tab-auth", 3], ["tab-body", 4]]);
const responseTabIndexByName = new Map([["tab-body", 1], ["tab-headers", 2]]);

export const $layoutState = map(initState);

export const toggleFolderPanel = () => {
    $layoutState.setKey("folderPanelVisible", !($layoutState.value?.folderPanelVisible ?? true));
    return true;
};

export const setFolderPanel = (visible: boolean) => {
    $layoutState.setKey("folderPanelVisible", visible);
    return true;
};

export const setViewMode = (mode: ViewMode) => {
    $layoutState.setKey("viewMode", mode);
}

export const setRequestTabActive = (index: number, allowToggle: boolean = true) => {
    if (allowToggle && !$layoutState.value?.requestTabPanelVisible) {
        $layoutState.setKey("requestTabPanelVisible", true);
        $layoutState.setKey("requestTabIndex", index);
    } else {
        if (allowToggle && $layoutState.value?.requestTabIndex === index) {
            $layoutState.setKey("requestTabPanelVisible", false);
            $layoutState.setKey("requestTabIndex", 0);
        } else {
            $layoutState.setKey("requestTabIndex", index);
        }
    }

    return true;
};

export const setRequestTabVisible = (visible: boolean) => {
    return true;
};

export const setRequestTabCount = (id: string, count: number) => {
    if (!requestTabIndexByName.has(id)) return;

    let tabIndex: number = requestTabIndexByName.get(id)!;
    let tabs = $layoutState.get().requestTabs;
    let tab = tabs[tabIndex - 1];
    tab.items = count;
    tabs[tabIndex - 1] = { ...tab };
    $layoutState.setKey("requestTabs", [...tabs]);
};

export const setResponseTabActive = (index: number, allowToggle: boolean = true) => {
    if (allowToggle && !$layoutState.value?.responseTabPanelVisible) {
        $layoutState.setKey("responseTabPanelVisible", true);
        $layoutState.setKey("responseTabIndex", index);
    } else {
        if (allowToggle && $layoutState.value?.responseTabIndex === index) {
            $layoutState.setKey("responseTabPanelVisible", false);
            $layoutState.setKey("responseTabIndex", 0);
        } else {
            $layoutState.setKey("responseTabIndex", index);
        }
    }

    return true;
};

export const setResponseTabVisible = (visible: boolean) => {
    return true;
};

export const setResponseTabCount = (id: string, count: number) => {
    if (!responseTabIndexByName.has(id)) return;

    let tabIndex: number = responseTabIndexByName.get(id)!;
    let tabs = $layoutState.get().responseTabs;
    let tab = tabs[tabIndex - 1];
    tab.items = count;
    tabs[tabIndex - 1] = { ...tab };
    $layoutState.setKey("responseTabs", [...tabs]);
};
