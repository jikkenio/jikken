import { createSignal, Show } from 'solid-js';
import { setRequestTabCount } from '../../../../stores/layoutState';
import { $editorState, type Compare, updateCompare, updateRequest, type HttpHeader, type Request, updateCompareState } from '../../../../stores/editorState';
import MonacoEditorSolid from '../../api/MonacoEditorSolid';
import { EntityType } from '../../../../stores/enum';
import { tippy } from '../../../TippySolid';

export const Body = () => {

    tippy;

    enum BodyType {
        None,
        Json,
    };

    type Body = {
        type: BodyType,
        content?: Object,
    };

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile];
    let currentTestFile = currentFile.type === EntityType.Test ? editorState.testFiles[currentFile.index] : undefined;

    const [body, setBody] = createSignal<Body>((() => {
        const body = currentTestFile?.testFile.request?.body;
        return { type: body ? BodyType.Json : BodyType.None, content: body };
    })());

    const [compareBody, setCompareBody] = createSignal<Body>((() => {
        const body = currentTestFile?.testFile.compare?.body;
        return { type: body ? BodyType.Json : BodyType.None, content: body };
    })());

    const [showCompare, setShowCompare] = createSignal(currentTestFile?.testFile.compare !== undefined);
    const [inheritCompare, setInheritCompare] = createSignal(currentTestFile?.compare?.inheritBody ?? false);

    $editorState.subscribe((state) => {
        let file = state.files[state.currentFile];

        // only update the data signal if the file changes
        if (file.id !== currentFile.id) {
            currentTestFile = currentFile.type === EntityType.Test ? state.testFiles[currentFile.index] : undefined;
            let body = currentTestFile?.testFile.request?.body;
            let compareBody = currentTestFile?.testFile.compare?.body;
            setBody({ type: body ? BodyType.Json : BodyType.None, content: body });
            setCompareBody({ type: compareBody ? BodyType.Json : BodyType.None, content: compareBody });
            setShowCompare(currentTestFile?.testFile.compare !== undefined);
            setInheritCompare(currentTestFile?.compare?.inheritBody ?? false);
            currentFile = file;
        }
    });

    const onTypeChange = (type: BodyType, compare: boolean = false) => {
        console.log("type change");
        let currentBody = compare ? compareBody() : body();
        currentBody.type = type;

        if (type === BodyType.Json) {
            currentBody.content = { field: "value" };
        } else {
            currentBody.content = undefined;
        }

        if (compare) {
            setCompareBody({ ...currentBody });
        } else {
            setBody({ ...currentBody });
        }

        setRequestTabCount("tab-body", type === BodyType.Json ? 1 : 0);
        updateBody(currentBody, compare);
        toggleContentType(type === BodyType.Json, compare);
    };

    const onBodyChange = (value: string, compare: boolean = false) => {
        console.log("body change");
        let currentBody = compare ? compareBody() : body();

        try {
            currentBody.content = JSON.parse(value);
            updateBody(currentBody, compare);
        } catch {
            console.log("invalid json, skipping update");
        }
    };

    const toggleContentType = (enabled: boolean, compare: boolean = false) => {
        let headers;
        if (compare) {
            let compare = { ...currentTestFile?.testFile.compare ?? {} as Compare };
            headers = compare.headers ?? [];
        } else {
            let request = { ...currentTestFile?.testFile.request ?? {} as Request };
            headers = request.headers ?? [];
        }

        let index = headers.findIndex(h => h.header.toLowerCase() === "content-type");
        console.log(`content header index ${index}`);
        console.log(`headers length ${headers.length}`);

        if (enabled && index === -1) {
            // add content type header if it's not there
            console.log("adding content type");
            headers.push({ header: "Content-Type", value: "application/json", generated: true });
            updateHeaders(headers, compare);
        } else if (!enabled && index > -1) {
            // remove content type header
            console.log(`removing content type at ${index}`);
            headers.splice(index, 1);
            updateHeaders(headers, compare);
        }
    };

    const updateHeaders = (headers: HttpHeader[], compare: boolean = false) => {
        if (compare) {
            let compare = { ...currentTestFile?.testFile.compare ?? {} as Compare };
            compare.headers = headers;
            updateCompare(compare);
        } else {
            let request = { ...currentTestFile?.testFile.request ?? {} as Request };
            request.headers = headers;
            updateRequest(request);
        }

        setRequestTabCount("tab-headers", headers.length);
    };

    const updateBody = (body: Body, compare: boolean = false) => {
        if (compare) {
            let compare = { ...currentTestFile?.testFile.compare ?? {} as Compare };
            compare.body = body.content;
            updateCompare(compare);
        } else {
            let request = { ...currentTestFile?.testFile.request ?? {} as Request };
            request.body = body.content;
            updateRequest(request);
        }
    };

    const toggleInherit = () => {
        let value = !inheritCompare();
        setInheritCompare(value);

        let compare = currentTestFile!.compare!;
        compare.inheritBody = value;
        updateCompareState({ ...compare });
    };

    const getCompareBody = () => {
        if (inheritCompare()) return body();
        return compareBody();
    };

    return (
        <div class="flex size-full min-w-0">
            <div class="flex size-full grid grid-cols-2 divide-x-[0.5px] divide-neutral-700">

                <div class="flex flex-col size-full min-h-0 pt-1"
                    classList={{
                        "col-span-2": !showCompare(),
                        "col-span-1": showCompare(),
                    }}>
                    <div class="flex-none px-3">
                        <div class="flex space-x-4 items-center text-neutral-500 text-xs font-medium py-2">
                            <div class="group flex items-center space-x-1">
                                <input type="radio" name="body-type" id="none" value={BodyType.None} checked={body().type === BodyType.None}
                                    class="peer text-indigo-600 bg-transparent size-[12px] cursor-pointer hover:border-indigo-600 checked:bg-indigo-700"
                                    onClick={(_) => onTypeChange(BodyType.None)} />
                                <label for="none" class="cursor-pointer group-hover:text-white peer-checked:text-neutral-300">none</label>
                            </div>
                            <div class="group flex flex-row items-center space-x-1">
                                <input type="radio" name="body-type" id="json" value={BodyType.Json} checked={body().type === BodyType.Json}
                                    class="peer text-indigo-600 bg-transparent size-[12px] cursor-pointer hover:border-indigo-600 checked:bg-indigo-700"
                                    onClick={(_) => onTypeChange(BodyType.Json)} />
                                <label for="json" class="cursor-pointer group-hover:text-white peer-checked:text-neutral-300">JSON</label>
                            </div>
                        </div>
                    </div>

                    <Show when={body().type === BodyType.Json}>
                        <div class="flex-auto min-h-0 px-3 pb-3">
                            <div class="size-full">
                                <MonacoEditorSolid value={body().content ? JSON.stringify(body().content!, undefined, 2) : undefined} language="json" onChange={(v) => onBodyChange(v, false)} />
                            </div>
                        </div>
                    </Show>
                </div>

                <Show when={showCompare()}>
                    <div class="flex flex-col size-full min-h-0 pt-1 col-span-1">
                        <div class="flex justify-between flex-none px-3">
                            <div class="flex space-x-4 items-center text-neutral-500 text-xs font-medium py-2">
                                <div class="group flex items-center space-x-1">
                                    <input type="radio"
                                        name="compare-body-type"
                                        id="compare-none"
                                        disabled={inheritCompare()}
                                        value={BodyType.None}
                                        checked={getCompareBody().type === BodyType.None}
                                        class="peer bg-transparent size-[12px]"
                                        classList={{
                                            "text-indigo-600 cursor-pointer hover:border-indigo-600 checked:bg-indigo-700": !inheritCompare(),
                                            "checked:bg-transparent checked:border-neutral-500": inheritCompare(),
                                        }}
                                        onClick={(_) => onTypeChange(BodyType.None, true)} />
                                    <label for="compare-none"
                                        classList={{
                                            "cursor-pointer group-hover:text-white peer-checked:text-neutral-300": !inheritCompare(),
                                            "text-neutral-500": inheritCompare(),
                                        }}>
                                        none
                                    </label>
                                </div>
                                <div class="group flex flex-row items-center space-x-1">
                                    <input type="radio"
                                        name="compare-body-type"
                                        id="compare-json"
                                        disabled={inheritCompare()}
                                        value={BodyType.Json}
                                        checked={getCompareBody().type === BodyType.Json}
                                        class="peer bg-transparent size-[12px]"
                                        classList={{
                                            "text-indigo-600 cursor-pointer hover:border-indigo-600 checked:bg-indigo-700": !inheritCompare(),
                                            "checked:bg-transparent checked:border-neutral-500": inheritCompare(),
                                        }}
                                        onClick={(_) => onTypeChange(BodyType.Json, true)} />
                                    <label for="compare-json"
                                        classList={{
                                            "cursor-pointer group-hover:text-white peer-checked:text-neutral-300": !inheritCompare(),
                                            "text-neutral-500": inheritCompare(),
                                        }}>
                                        JSON
                                    </label>
                                </div>
                            </div>
                            <div class="group cursor-pointer mt-[-2px]"
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

                        <Show when={getCompareBody().type === BodyType.Json}>
                            <div class="flex-auto min-h-0 px-3 pb-3">
                                <div class="size-full">
                                    <MonacoEditorSolid value={getCompareBody().content ? JSON.stringify(getCompareBody().content!, undefined, 2) : undefined}
                                        language="json"
                                        readonly={inheritCompare()}
                                        onChange={(v) => onBodyChange(v, true)} />
                                </div>
                            </div>
                        </Show>
                    </div>
                </Show>
            </div>
        </div>
    );
};
