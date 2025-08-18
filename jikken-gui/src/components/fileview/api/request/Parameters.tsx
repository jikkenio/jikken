import { createSignal, For, Show } from 'solid-js';
import { setRequestTabCount } from '../../../../stores/layoutState';
import { $editorState, type Compare, updateCompare, updateRequest, type HttpParameter, type Request, updateCompareState } from '../../../../stores/editorState';
import { EntityType } from '../../../../stores/enum';
import { tippy } from '../../../TippySolid';

export const Parameters = () => {

    tippy;

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile];
    let currentTestFile = currentFile.type === EntityType.Test ? editorState.testFiles[currentFile.index] : undefined;

    const [params, setParams] = createSignal<HttpParameter[]>((() => {
        const params = currentTestFile?.testFile.request?.params ?? [];
        return [...params, { param: "", value: "", generated: false }];
    })());

    const [compareParams, setCompareParams] = createSignal<HttpParameter[]>((() => {
        const params = currentTestFile?.testFile.compare?.params ?? [];
        return [...params, { param: "", value: "", generated: false }];
    })());

    const [showCompare, setShowCompare] = createSignal(currentTestFile?.testFile.compare !== undefined);
    const [inheritCompare, setInheritCompare] = createSignal(currentTestFile?.compare?.inheritParams ?? false);

    $editorState.subscribe((state) => {
        currentFile = state.files[state.currentFile];
        currentTestFile = currentFile.type === EntityType.Test ? state.testFiles[currentFile.index] : undefined;
        const params = currentTestFile?.testFile.request?.params ?? [];
        const compareParams = currentTestFile?.testFile.compare?.params ?? [];
        setParams([...params, { param: "", value: "", generated: false }]);
        setCompareParams([...compareParams, { param: "", value: "", generated: false }]);
        setShowCompare(currentTestFile?.testFile.compare !== undefined);
        setInheritCompare(currentTestFile?.compare?.inheritParams ?? false);
    });

    const onParamInput = (index: number, compare: boolean = false) => {
        let currentParams = compare ? compareParams() : params();

        // if the last row is not empty, add another row
        if (index === currentParams.length - 1) {
            if (compare) {
                setCompareParams([...currentParams, { param: "", value: "", generated: false }]);
            } else {
                setParams([...currentParams, { param: "", value: "", generated: false }]);
            }
            setRequestTabCount("tab-params", currentParams.length);
        }
    };

    const onParamChange = (index: number, param: HttpParameter, compare: boolean = false) => {
        console.log(`saving param at index ${index}`);
        let currentParams = [...(compare ? compareParams() : params())];
        currentParams[index] = param;

        // if we emptied a row and it's not the last, delete it
        if (param.param === "" && param.value === "" && index < currentParams.length - 1) {
            console.log(`deleting param at ${index}`)
            currentParams.splice(index, 1);
            if (compare) {
                setCompareParams(currentParams);
            } else {
                setParams(currentParams);
            }

            setRequestTabCount("tab-params", currentParams.length - 1);
        }

        updateParams(currentParams, compare);
    };

    const deleteParam = (index: number, compare: boolean = false) => {
        console.log(`deleting param at ${index}`);
        let currentParams = [...(compare ? compareParams() : params())];
        currentParams.splice(index, 1);
        if (compare) {
            setCompareParams(currentParams);
        } else {
            setParams(currentParams);
        }

        setRequestTabCount("tab-params", currentParams.length - 1);
        updateParams(currentParams, compare);
    };

    const updateParams = (params: HttpParameter[], compare: boolean = false) => {
        params.splice(-1, 1);

        if (compare) {
            let compare = { ...currentTestFile?.testFile.compare ?? {} as Compare };
            compare.params = params;
            updateCompare(compare);
        } else {
            let request = { ...currentTestFile?.testFile.request ?? {} as Request };
            request.params = params;
            updateRequest(request);
        }
    };

    const toggleInherit = () => {
        let value = !inheritCompare();
        setInheritCompare(value);

        let compare = currentTestFile!.compare!;
        compare.inheritParams = value;
        updateCompareState({ ...compare });
    };

    const getCompareParams = () => {
        if (!inheritCompare()) return compareParams();
        let requestParams = [...params()];
        requestParams.splice(-1);
        return requestParams;
    };

    return (
        <div id="tab-params-panel" class="size-full overflow-y-auto">
            <div class="grid grid-cols-2 size-full divide-x-[0.5px] divide-neutral-700">
                <ul class="p-3 pr-1"
                    classList={{
                        "col-span-2": !showCompare(),
                        "col-span-1": showCompare(),
                    }}>
                    <div class="h-5" />
                    <For each={params()}>
                        {(param, index) => (
                            <li class="flex flex-row group text-neutral-300">
                                <div class="flex flex-row flex-1 items-center border border-1 border-b-0 group-last:border-b border-neutral-700 focus-within:ring-2 focus-within:ring-inset focus-within:ring-indigo-600">
                                    <input placeholder="Key"
                                        spellcheck={false}
                                        autocorrect="off"
                                        value={param.param}
                                        onInput={(_) => onParamInput(index())}
                                        onChange={(e) => onParamChange(index(), { param: e.currentTarget.value, value: param.value, generated: false })}
                                        class="grow text-sm bg-transparent pl-2 p-1 border-0 placeholder:text-neutral-500 focus-within:ring-0"
                                    />
                                    <Show when={param.generated}>
                                        <span class="flex-none p-1 pr-1.5 text-neutral-400 hover:text-indigo-400" title="Auto-generated value">
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
                                    value={param.value}
                                    onInput={(_) => onParamInput(index())}
                                    onChange={(e) => onParamChange(index(), { param: param.param, value: e.currentTarget.value, generated: false })}
                                    class="flex-1 text-sm bg-transparent pl-2 p-1 border-1 border-l-0 border-b-0 group-last:border-b border-neutral-700 placeholder:text-neutral-500 focus-within:ring-2 focus-within:ring-inset focus-within:ring-indigo-600"
                                />
                                <div
                                    class="flex-none w-8 p-2 text-neutral-500 cursor-pointer group-last:cursor-default group-last:pointer-events-none hover:text-red-500"
                                    onClick={() => deleteParam(index())}
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

                <Show when={showCompare()}>
                    <div class="p-3 col-span-1">
                        <div class="flex justify-end h-5">
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
                        <ul>
                            <For each={getCompareParams()}>
                                {(param, index) => (
                                    <li class="flex flex-row group text-neutral-300">
                                        <div class="flex flex-row flex-1 items-center border border-1 border-b-0 group-last:border-b border-neutral-700 focus-within:ring-2 focus-within:ring-inset focus-within:ring-indigo-600">
                                            <input placeholder="Key"
                                                spellcheck={false}
                                                autocorrect="off"
                                                disabled={inheritCompare()}
                                                value={param.param}
                                                onInput={(_) => onParamInput(index(), true)}
                                                onChange={(e) => onParamChange(index(), { param: e.currentTarget.value, value: param.value, generated: false }, true)}
                                                class="grow text-sm bg-transparent pl-2 p-1 border-0 placeholder:text-neutral-500 focus-within:ring-0 disabled:text-neutral-500"
                                            />
                                            <Show when={param.generated}>
                                                <span class="flex-none p-1 pr-1.5 text-neutral-400 hover:text-indigo-400" title="Auto-generated value">
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
                                            disabled={inheritCompare()}
                                            value={param.value}
                                            onInput={(_) => onParamInput(index(), true)}
                                            onChange={(e) => onParamChange(index(), { param: param.param, value: e.currentTarget.value, generated: false }, true)}
                                            class="flex-1 text-sm bg-transparent pl-2 p-1 border-1 border-l-0 border-b-0 group-last:border-b border-neutral-700 placeholder:text-neutral-500 focus-within:ring-2 focus-within:ring-inset focus-within:ring-indigo-600 disabled:text-neutral-500"
                                        />
                                        <div
                                            class="flex-none w-8 p-2 text-neutral-500 cursor-pointer group-last:cursor-default group-last:pointer-events-none hover:text-red-500"
                                            onClick={() => deleteParam(index(), true)}
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
                </Show>
            </div>
        </div>
    );
};
