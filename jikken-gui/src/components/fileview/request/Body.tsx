import { createSignal, Show } from 'solid-js';
import { setRequestTabCount } from '../../../stores/layoutState';
import { $editorState, updateRequest, type HttpHeader, type Request } from '../../../stores/editorState';
import MonacoEditorSolid from '../MonacoEditorSolid';

export const Body = () => {

    enum BodyType {
        None,
        Json,
    };

    type Body = {
        type: BodyType,
        content?: Object,
    };

    let editorState = $editorState.get();
<<<<<<< HEAD
    let currentFile = editorState.files[editorState.currentFile];

    let [body, setBody] = createSignal({ type: currentFile.testFile.request?.body ? BodyType.Json : BodyType.None, content: currentFile.testFile.request?.body });

    $editorState.subscribe((state) => {
        let file = state.files[state.currentFile];

        // only update the data signal if the file changes
        if (file.id !== currentFile.id) {
            let stateBody = file.testFile.request?.body;
            setBody({ type: stateBody ? BodyType.Json : BodyType.None, content: stateBody });
            currentFile = file;
        }
=======
    let currentFile = editorState.files[editorState.currentFile].testFile;

    let [body, setBody] = createSignal({ type: currentFile.request?.body ? BodyType.Json : BodyType.None, content: currentFile.request?.body });

    $editorState.subscribe((state) => {
        let file = state.files[state.currentFile].testFile;
        let stateBody = file.request?.body;
        setBody({ type: stateBody ? BodyType.Json : BodyType.None, content: stateBody });
        currentFile = file;
>>>>>>> 330f865 (JK-594: initial move of jikken-gui project into repo)
    });

    const onTypeChange = (type: BodyType) => {
        console.log("type change");
        let currentBody = body();
        currentBody.type = type;

        if (type === BodyType.Json) {
            currentBody.content = { field: "value" };
        } else {
            currentBody.content = undefined;
        }

        console.log(currentBody);
        setBody({ ...currentBody });
        setRequestTabCount("tab-body", type === BodyType.Json ? 1 : 0);
        updateBody(currentBody);
        toggleContentType(type === BodyType.Json);
    };

    const onBodyChange = (value: string) => {
        console.log("body change");
        let currentBody = body();

        try {
            currentBody.content = JSON.parse(value);
            updateBody(currentBody);
        } catch {
            console.log("invalid json, skipping update");
        }
    };

    const toggleContentType = (enabled: boolean) => {
<<<<<<< HEAD
        let request = { ...currentFile.testFile.request ?? {} as Request };
=======
        let request = { ...currentFile.request ?? {} as Request };
>>>>>>> 330f865 (JK-594: initial move of jikken-gui project into repo)
        let headers = request.headers ?? [];
        let index = headers.findIndex(h => h.header.toLowerCase() === "content-type");
        console.log(`content header index ${index}`);
        console.log(`headers length ${headers.length}`);

        if (enabled && index === -1) {
            // add content type header if it's not there
            console.log("adding content type");
            headers.push({ header: "Content-Type", value: "application/json", generated: true });
            updateHeaders(headers);
        } else if (!enabled && index > -1) {
            // remove content type header
            console.log(`removing content type at ${index}`);
            headers.splice(index, 1);
            updateHeaders(headers);
        }
    };

    const updateHeaders = (headers: HttpHeader[]) => {
<<<<<<< HEAD
        let request = { ...currentFile.testFile.request ?? {} as Request };
=======
        let request = { ...currentFile.request ?? {} as Request };
>>>>>>> 330f865 (JK-594: initial move of jikken-gui project into repo)
        request.headers = headers;
        updateRequest(request);
        setRequestTabCount("tab-headers", headers.length);
    };

    const updateBody = (body: Body) => {
<<<<<<< HEAD
        let request = { ...currentFile.testFile.request ?? {} as Request };
=======
        let request = { ...currentFile.request ?? {} as Request };
>>>>>>> 330f865 (JK-594: initial move of jikken-gui project into repo)
        request.body = body.content;
        updateRequest(request);
    };

    return (
        <div id="tab-body-panel" class="p-3 pt-0 size-full">
            <div class="flex space-x-4 items-center text-neutral-500 mb-3 text-xs font-medium">
                <div class="group flex items-center space-x-1">
                    <input type="radio" name="body-type" id="none" value={BodyType.None} checked={body().type === BodyType.None}
                        class="peer text-indigo-600 bg-transparent size-[12px] cursor-pointer hover:border-indigo-600"
                        onClick={(_) => onTypeChange(BodyType.None)} />
                    <label for="none" class="cursor-pointer group-hover:text-white peer-checked:text-neutral-300">none</label>
                </div>
                <div class="group flex flex-row items-center space-x-1">
                    <input type="radio" name="body-type" id="json" value={BodyType.Json} checked={body().type === BodyType.Json}
                        class="peer text-indigo-600 bg-transparent size-[12px] cursor-pointer hover:border-indigo-600"
                        onClick={(_) => onTypeChange(BodyType.Json)} />
                    <label for="json" class="cursor-pointer group-hover:text-white peer-checked:text-neutral-300">JSON</label>
                </div>
            </div>

            <Show when={body().type === BodyType.Json}>
                <div class="flex flex-auto w-full">
                    <MonacoEditorSolid value={body().content ? JSON.stringify(body().content!, undefined, 2) : undefined} language="json" onChange={onBodyChange} />
                </div>
            </Show>
        </div>
    );
}
