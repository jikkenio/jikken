import { createSignal, For } from 'solid-js';
import { $editorState, type GlobalVariable } from '../../../stores/editorState';
import { EntityType } from '../../../stores/enum';

export const ConfigView = () => {

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile];
    let currentConfigFile = currentFile.type === EntityType.Config ? editorState.configFiles[currentFile.index] : undefined;

    let [config, setConfig] = createSignal(currentConfigFile);
    let [globals, setGlobals] = createSignal([...currentConfigFile?.globals ?? [], { key: "", value: "" }])

    $editorState.subscribe((state) => {
        let currentFile = state.files[state.currentFile];
        let currentConfigFile = currentFile.type === EntityType.Config ? state.configFiles[currentFile.index] : undefined;
        setConfig(currentConfigFile);
        setGlobals([...currentConfigFile?.globals ?? [], { key: "", value: "" }])
    });

    const toggleBypassCertVerification = (value: boolean) => {
        currentConfigFile!.bypassCertVerification = value;
        console.log("set config: ", currentConfigFile);
        setConfig(currentConfigFile);
    }

    const toggleContinueOnFailure = (value: boolean) => {
        currentConfigFile!.continueOnFailure = value;
        console.log("set config: ", currentConfigFile);
        setConfig(currentConfigFile);
    }

    const updateApiKey = (value: string) => {
        currentConfigFile!.apiKey = value;
        console.log("set config: ", currentConfigFile);
        setConfig(currentConfigFile);
    }

    const updateEnvironment = (value: string) => {
        currentConfigFile!.environment = value;
        console.log("set config: ", currentConfigFile);
        setConfig(currentConfigFile);
    }

    const onGlobalInput = (index: number) => {
        let currentGlobals = globals();

        // if the last row is not empty, add another row
        if (index === currentGlobals.length - 1) {
            setGlobals([...currentGlobals, { key: "", value: "" }]);
        }
    };

    const onGlobalChange = (index: number, global: GlobalVariable) => {
        console.log(`saving global variable at index ${index}`);
        let currentGlobals = globals();
        currentGlobals[index] = global;

        // if we emptied a row and it's not the last, delete it
        if (global.key === "" && global.value === "" && index < currentGlobals.length - 1) {
            deleteGlobal(index);
            return;
        }

        setGlobals([...currentGlobals]);
        // updateGlobals(currentGlobals);
    }

    const deleteGlobal = (index: number) => {
        console.log(`deleting global variable at ${index}`);
        let currentGlobals = globals();
        currentGlobals.splice(index, 1);

        setGlobals([...currentGlobals]);
        // updateGlobals(currentGlobals);
    };

    return (
        <div class="p-3 flex flex-auto flex-col">
            <div class="flex flex-col">
                <div class="text-neutral-400 my-2 text-sm font-medium">Configurations</div>
                <ul class="w-full p-1">
                    <li class="grid grid-cols-5 group text-neutral-300 h-[30px]">
                        <div class="col-span-2 text-sm bg-transparent my-auto">
                            API Key
                        </div>
                        <div class="col-span-3 text-sm bg-transparent w-75 my-auto">
                            <input placeholder="API Key"
                                spellcheck={false}
                                autocorrect="off"
                                value={config()?.apiKey || ""}
                                onChange={(e) => updateApiKey(e.currentTarget.value)}
                                class="w-full text-sm bg-transparent pl-2 p-1 border-1 border-neutral-700 placeholder:text-neutral-500 focus-within:ring-2 focus-within:ring-inset focus-within:ring-indigo-600"
                            />
                        </div>
                    </li>
                    <li class="grid grid-cols-5 group text-neutral-300 h-[30px]">
                        <div class="col-span-2 text-sm bg-transparent my-auto">
                            Bypass Certificate Verification
                        </div>
                        <div class="col-span-3 flex space-x-4 text-sm bg-transparent my-auto">
                            <div class="group/true flex items-center space-x-1">
                                <input type="radio" name="bypass-cert-verification" id="bypass-true" value="true" checked={config()?.bypassCertVerification}
                                    class="peer text-indigo-600 bg-transparent size-[12px] cursor-pointer group-hover/true:border-indigo-600 checked:bg-indigo-700"
                                    onClick={(_) => toggleBypassCertVerification(true)} />
                                <label for="bypass-true" class="cursor-pointer group-hover/true:text-white peer-checked:text-neutral-300">true</label>
                            </div>
                            <div class="group/false flex flex-row items-center space-x-1">
                                <input type="radio" name="bypass-cert-verification" id="bypass-false" value="false" checked={!config()?.bypassCertVerification}
                                    class="peer text-indigo-600 bg-transparent size-[12px] cursor-pointer group-hover/false:border-indigo-600 checked:bg-indigo-700"
                                    onClick={(_) => toggleBypassCertVerification(false)} />
                                <label for="bypass-false" class="cursor-pointer group-hover/false:text-white peer-checked:text-neutral-300">false</label>
                            </div>
                        </div>
                    </li>
                    <li class="grid grid-cols-5 group text-neutral-300 h-[30px]">
                        <div class="col-span-2 text-sm bg-transparent my-auto">
                            Continue on Failure
                        </div>
                        <div class="col-span-3 flex space-x-4 text-sm bg-transparent my-auto">
                            <div class="group/true flex items-center space-x-1">
                                <input type="radio" name="continue-on-faikure" id="continue-true" value="true" checked={config()?.continueOnFailure}
                                    class="peer text-indigo-600 bg-transparent size-[12px] cursor-pointer group-hover/true:border-indigo-600 checked:bg-indigo-700"
                                    onClick={(_) => toggleContinueOnFailure(true)} />
                                <label for="continue-true" class="cursor-pointer group-hover/true:text-white peer-checked:text-neutral-300">true</label>
                            </div>
                            <div class="group/false flex flex-row items-center space-x-1">
                                <input type="radio" name="continue-on-failure" id="continue-false" value="false" checked={!config()?.continueOnFailure}
                                    class="peer text-indigo-600 bg-transparent size-[12px] cursor-pointer group-hover/false:border-indigo-600 checked:bg-indigo-700"
                                    onClick={(_) => toggleContinueOnFailure(false)} />
                                <label for="continue-false" class="cursor-pointer group-hover/false:text-white peer-checked:text-neutral-300">false</label>
                            </div>
                        </div>
                    </li>
                    <li class="grid grid-cols-5 group text-neutral-300 h-[30px]">
                        <div class="col-span-2 text-sm bg-transparent my-auto">
                            Environment
                        </div>
                        <div class="col-span-3 text-sm bg-transparent w-75 my-auto">
                            <input placeholder="Environment"
                                spellcheck={false}
                                autocorrect="off"
                                value={config()?.environment || ""}
                                onChange={(e) => updateEnvironment(e.currentTarget.value)}
                                class="w-full text-sm bg-transparent pl-2 p-1 border-1 border-neutral-700 placeholder:text-neutral-500 focus-within:ring-2 focus-within:ring-inset focus-within:ring-indigo-600"
                            />
                        </div>
                    </li>
                </ul>
            </div>
            <div class="flex flex-col mt-10">
                <div class="text-neutral-400 mb-3 text-sm font-medium">Global Variables</div>
                <ul class="w-full">
                    <For each={globals()}>
                        {(global, index) => (
                            <li class="flex flex-row group text-neutral-300">
                                <input placeholder="Key"
                                    spellcheck={false}
                                    autocorrect="off"
                                    value={global.key}
                                    onInput={(_) => onGlobalInput(index())}
                                    onChange={(e) => onGlobalChange(index(), { key: e.currentTarget.value, value: global.value })}
                                    class="flex-1 text-sm bg-transparent pl-2 p-1 border-1 border-b-0 group-last:border-b border-neutral-700 placeholder:text-neutral-500 focus-within:ring-2 focus-within:ring-inset focus-within:ring-indigo-600"
                                />
                                <input placeholder="Value"
                                    spellcheck={false}
                                    autocorrect="off"
                                    value={global.value}
                                    onInput={(_) => onGlobalInput(index())}
                                    onChange={(e) => onGlobalChange(index(), { key: global.key, value: e.currentTarget.value })}
                                    class="flex-1 text-sm bg-transparent pl-2 p-1 border-1 border-l-0 border-b-0 group-last:border-b border-neutral-700 placeholder:text-neutral-500 focus-within:ring-2 focus-within:ring-inset focus-within:ring-indigo-600"
                                />
                                <div
                                    class="flex-none w-8 p-2 text-neutral-500 cursor-pointer group-last:cursor-default group-last:pointer-events-none hover:text-red-500"
                                    onClick={[deleteGlobal, index()]}
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
        </div>
    );
}
