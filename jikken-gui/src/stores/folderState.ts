import { atom } from 'nanostores';
import { invoke } from "@tauri-apps/api/core";
import { $editorState, type File } from './editorState';
import { EntityType } from './enum';

export type FolderResult = {
    name: string,
    path: string,
    isDirectory: boolean,
    entities: FolderResult[]
};

export type FolderEntity = {
    name: string,
    path: string,
    type: EntityType,
    indentationLevel: number,
    isHidden: boolean,
    isExpanded: boolean,
    isLoaded: boolean,
};

export type FolderViewState = {
    entities: FolderEntity[],
    activeIndex: number,
};

const initState: FolderViewState = {
    entities: [],
    activeIndex: -1,
};

export const $folderState = atom(initState);

const sortEntities = (entities: FolderEntity[]) => {
    entities.sort((a, b) => a.path.localeCompare(b.path));
}

const generateEntities = (folderResult: FolderResult, startingIndentation: number = 0) => {
    if (!folderResult) return [];
    let entities: FolderEntity[] = [];

    // find the common path prefix so we can prune it out when determining indentation
    let index = folderResult.path.indexOf(folderResult.name);
    let commonPath = folderResult.path.slice(0, index);

    let currentEntities = [folderResult];
    let nextEntities;
    while (currentEntities) {
        nextEntities = [];
        for (var entity of currentEntities) {
            // use the number of directories in the path to determine the indentation
            let pathParts = entity.path.replace(commonPath, "").split("/");

            entities.push({
                name: entity.name,
                path: entity.path,
                type: getEntityType(entity),
                indentationLevel: pathParts.length + startingIndentation - 1,
                isHidden: false,
                isExpanded: entity.isDirectory && entity.path === folderResult.path,
                isLoaded: entity.isDirectory && entity.path === folderResult.path,
            });

            if (entity.entities.length > 0) {
                nextEntities.push(...entity.entities);
            }
        }

        if (nextEntities.length === 0) {
            break;
        }

        // move down to the next level of entities
        currentEntities = nextEntities;
    }

    return entities;
};

const getEntityType = (entity: FolderResult) => {
    if (entity.isDirectory) return EntityType.Directory;
    if (entity.name.endsWith(".jikken")) return EntityType.Config;
    return EntityType.Test;
}

export const selectEntity = (index: number) => {
    console.log("selecting entity at index ", index);
    let state = $folderState.get();
    state.activeIndex = index;
    $folderState.set({ ...state });
}

export const selectEntityPath = (path: string | undefined) => {
    console.log("selecting entity with path ", path);
    let state = $folderState.get()

    if (path) {
        let index = state.entities.findIndex((e) => e.path === path);
        state.activeIndex = index;
    } else {
        state.activeIndex = -1;
    }

    $folderState.set({ ...state });
}

export const toggleFolder = async (folder: FolderEntity) => {
    console.log("toggling folder: ", folder.name);
    let state = $folderState.get()
    let entities = state.entities;
    let matchingEntity = entities.find((e) => e.path === folder.path)!;
    matchingEntity.isExpanded = !folder.isExpanded;

    // toggle all entities inside the folder (if data has been loaded previously)
    if (matchingEntity.isLoaded) {
        entities.filter((m) => m.path !== folder.path && m.path.startsWith(folder.path))
            .forEach((m) => {
                m.isHidden = !folder.isExpanded;
                if (m.isLoaded) {
                    m.isExpanded = true;
                }
            });
    }

    $folderState.set(JSON.parse(JSON.stringify(state)));

    // load the toggled folder, if it hasn't been loaded
    if (!matchingEntity.isLoaded) {
        console.log("loading folder: ", folder.path);
        await loadFolder(matchingEntity.path);
    }
};

export const openFolderDialog = async () => {
    let result: FolderResult = await invoke("open_folder_dialog");
    console.log("open folder result: ", result);
    if (!result) return;

    let state = $folderState.get();
    if (state.entities.some((e) => e.path === result.path)) {
        console.log("folder already open")
        return;
    }

    // store the active file path (if any) so we can update the index after adding entities
    let selectedPath = getSelectedPath(state);

    let updatedEntities = [...state.entities, ...generateEntities(result)];
    sortEntities(updatedEntities);

    // update the active index to point to the same file as before
    let newIndex = selectedPath ? updatedEntities.findIndex((e) => e.path === selectedPath) : -1;

    $folderState.set({
        entities: JSON.parse(JSON.stringify(updatedEntities)),
        activeIndex: newIndex,
    });
};

export const loadFolder = async (path: string) => {
    let result: FolderResult = await invoke("open_folder_path", { path: path });
    console.log("open folder result: ", result);
    if (!result) return;

    let state = $folderState.get();
    let entities = state.entities;

    // store the active file path (if any) so we can update the index after adding entities
    let selectedPath = getSelectedPath(state);

    let existingEntity = entities.find((e) => e.path === path)!;

    // remove all existing entities that fall under the path, as they will be reloaded
    entities = entities.filter((e) => !e.path.includes(path));

    let updatedEntities = [...entities, ...generateEntities(result, existingEntity.indentationLevel)];
    sortEntities(updatedEntities);

    // update the active index to point to the same file as before
    let newIndex = selectedPath ? updatedEntities.findIndex((e) => e.path === selectedPath) : -1;

    $folderState.set({
        entities: JSON.parse(JSON.stringify(updatedEntities)),
        activeIndex: newIndex,
    });
};

export const removeFolder = (folder: FolderEntity) => {
    console.log("removing folder at path: ", folder.path);
    let state = $folderState.get();
    let entities = state.entities;

    // store the active file path (if any) so we can update the index after removing entities
    let selectedPath = getSelectedPath(state);

    entities = entities.filter((e) => !e.path.startsWith(folder.path));

    // update the active index to point to the same file as before
    let newIndex = selectedPath ? entities.findIndex((e) => e.path === selectedPath) : -1;

    $folderState.set({
        entities: JSON.parse(JSON.stringify(entities)),
        activeIndex: newIndex,
    });
};

export const addSavedFile = async (file: File) => {
    console.log("adding file to folder view: ", file.path);
    let state = $folderState.get();
    let entities = state.entities;
    if (entities.some((e) => e.path === file.path)) {
        console.log("file already in view");
        return;
    }

    // find entities in the same path hierarchy
    let matchingEntities = entities.filter((e) => file.path.startsWith(e.path));

    // if there are none, file is not added to the view
    if (matchingEntities.length === 0) {
        console.log("file does not belong in any open folders");
        return;
    }

    // expand all loaded collapsed ancestor directories
    // (only need to do the highest level found as it will expand all its children)
    let highestCollapsedParent = matchingEntities.find((e) => e.type === EntityType.Directory && e.isLoaded && !e.isExpanded);
    if (highestCollapsedParent) {
        toggleFolder(highestCollapsedParent);
    }

    // find any ancestor directories that haven't been loaded
    let unloadedEntities = matchingEntities.filter((e) => !e.isLoaded);

    // always reload the closest ancestor (in case of newly created folders)
    let closestEntity = matchingEntities[matchingEntities.length - 1];
    if (closestEntity.isLoaded) {
        unloadedEntities.push(closestEntity);
    }

    // iterate through and load any unloaded ancestors
    while (unloadedEntities.length > 0) {
        for (var unloadedEntity of unloadedEntities) {
            console.log("loading ancestor: ", unloadedEntity.path);
            await loadFolder(unloadedEntity.path);
        }

        // reload state to catch any nested folders created during file save
        unloadedEntities = $folderState.get().entities.filter((e) => file.path.startsWith(e.path) && e.type === EntityType.Directory && !e.isLoaded);
    }

    console.log("all ancestors loaded");
    selectEntityPath(file.path);
};

const getSelectedPath = (state: FolderViewState) => {
    let selectedPath = state.entities[state.activeIndex]?.path;
    if (selectedPath) return selectedPath;

    let editorState = $editorState.get();
    return editorState.files[editorState.currentFile].file?.path;
};
