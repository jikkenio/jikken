import { createSignal, For, Show } from 'solid-js';
import { setRequestTabCount } from '../../../stores/layoutState';
import { $editorState, updateRequest, type HttpParameter, type Request } from '../../../stores/editorState';

export const Parameters = () => {

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile].testFile;
    let [params, setParams] = createSignal(currentFile.request?.params ?? []);

    $editorState.subscribe((state) => {
        let file = state.files[state.currentFile].testFile;
        let stateParams = file.request?.params ?? [];
        setParams([...stateParams, { param: "", value: "", generated: false }]);
        currentFile = file;
    });

    const onParamInput = (index: number) => {
        let currentParams = params();

        // if the last row is not empty, add another row
        if (index === currentParams.length - 1) {
            setParams([...currentParams, { param: "", value: "", generated: false }]);
            setRequestTabCount("tab-params", currentParams.length);
        }
    };

    const onParamChange = (index: number, param: HttpParameter) => {
        console.log(`saving param at index ${index}`);
        let currentParams = [...params()];
        currentParams[index] = param;

        // if we emptied a row and it's not the last, delete it
        if (param.param === "" && param.value === "" && index < currentParams.length - 1) {
            console.log(`deleting param at ${index}`)
            currentParams.splice(index, 1);
            setParams(currentParams);
            setRequestTabCount("tab-params", currentParams.length - 1);
        }

        updateParams(currentParams);
    }

    const deleteParam = (index: number) => {
        console.log(`deleting param at ${index}`);
        let currentParams = [...params()];
        currentParams.splice(index, 1);
        setParams(currentParams);
        setRequestTabCount("tab-params", currentParams.length - 1);
        updateParams(currentParams);
    };

    const updateParams = (params: HttpParameter[]) => {
        let request = { ...currentFile.request ?? {} as Request };
        params.splice(-1, 1);
        request.params = params;
        updateRequest(request);
    }

    return (
        <div id="tab-params-panel" class="p-3 pt-1">
            <div class="text-neutral-400 mb-2 text-sm font-medium">Query Parameters</div>

            <ul>
                <For each={params()}>
                    {(param, index) => (
                        <li class="flex flex-row group text-neutral-300">
                            <div class="flex flex-row flex-1 items-center border border-1 border-b-0 group-last:border-b border-neutral-700 focus-within:ring-2 focus-within:ring-inset focus-within:ring-indigo-600">
                                <input placeholder="Key"
<<<<<<< HEAD
                                    spellcheck={false}
                                    autocorrect="off"
=======
>>>>>>> 330f865 (JK-594: initial move of jikken-gui project into repo)
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
<<<<<<< HEAD
                                spellcheck={false}
                                autocorrect="off"
=======
>>>>>>> 330f865 (JK-594: initial move of jikken-gui project into repo)
                                value={param.value}
                                onInput={(_) => onParamInput(index())}
                                onChange={(e) => onParamChange(index(), { param: param.param, value: e.currentTarget.value, generated: false })}
                                class="flex-1 text-sm bg-transparent pl-2 p-1 border-1 border-l-0 border-b-0 group-last:border-b border-neutral-700 placeholder:text-neutral-500 focus-within:ring-2 focus-within:ring-inset focus-within:ring-indigo-600"
                            />
                            <div
                                class="flex-none w-8 p-2 text-neutral-500 cursor-pointer group-last:cursor-default group-last:pointer-events-none hover:text-red-500"
                                onClick={[deleteParam, index()]}
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
    );
}
