import { createSignal, For } from 'solid-js';
import { $editorState, updateConfigFile } from '../../../stores/editorState';
import { EntityType } from '../../../stores/enum';

export const ConfigView = () => {

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile];
    let currentConfigFile = currentFile.type === EntityType.Config ? editorState.configFiles[currentFile.index] : undefined;

    const listFromGlobals = (globals: Map<string, string> | undefined) => {
        let list: [string, string][] = [];
        if (globals) {
            list = list.concat(Array.from(globals!.entries()));
        }

        list.push(["", ""]);
        return list;
    };

    let [config, setConfig] = createSignal(currentConfigFile);
    let [globals, setGlobals] = createSignal(listFromGlobals(currentConfigFile?.globals));

    $editorState.subscribe((state) => {
        let currentFile = state.files[state.currentFile];
        let currentConfigFile = currentFile.type === EntityType.Config ? state.configFiles[currentFile.index] : undefined;
        setConfig(currentConfigFile);
        setGlobals(listFromGlobals(currentConfigFile?.globals));
    });

    const toggleBypassCertVerification = (value: boolean) => {
        let current = config();
        if (!current!.settings) current!.settings = {};

        current!.settings!.bypassCertVerification = value;
        console.log("set config: ", current);
        setConfig(current);
        updateConfig();
    }

    const toggleContinueOnFailure = (value: boolean) => {
        let current = config();
        if (!current!.settings) current!.settings = {};

        current!.settings!.continueOnFailure = value;
        console.log("set config: ", current);
        setConfig(current);
        updateConfig();
    }

    const updateApiKey = (value: string) => {
        let current = config();
        if (!current!.settings) current!.settings = {};

        current!.settings!.apiKey = value;
        console.log("set config: ", current);
        setConfig(current);
        updateConfig();
    }

    const updateEnvironment = (value: string) => {
        let current = config();
        if (!current!.settings) current!.settings = {};

        current!.settings!.environment = value;
        console.log("set config: ", current);
        setConfig(current);
        updateConfig();
    }

    const onGlobalInput = (index: number) => {
        let currentGlobals = globals();

        // if the last row is not empty, add another row
        if (index === currentGlobals.length - 1) {
            setGlobals([...currentGlobals, ["", ""]]);
        }
    };

    const onGlobalChange = (index: number, global: [string, string]) => {
        console.log(`saving global variable at index ${index}`);
        let currentGlobals = globals();
        currentGlobals[index] = global;

        // if we emptied a row and it's not the last, delete it
        if (global[0] === "" && global[1] === "" && index < currentGlobals.length - 1) {
            deleteGlobal(index);
            return;
        }

        setGlobals([...currentGlobals]);
        updateConfig();
    }

    const deleteGlobal = (index: number) => {
        console.log(`deleting global variable at ${index}`);
        let currentGlobals = globals();
        currentGlobals.splice(index, 1);

        setGlobals([...currentGlobals]);
        updateConfig();
    };

    const updateConfig = () => {
        let file = config();
        if (!config) return;

        let envs = globals();
        envs.splice(-1, 1);
        file!.globals = new Map(envs);
        updateConfigFile({ ...file! });
    }

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
                                value={config()?.settings?.apiKey || ""}
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
                                <input type="radio" name="bypass-cert-verification" id="bypass-true" value="true" checked={config()?.settings?.bypassCertVerification}
                                    class="peer text-indigo-600 bg-transparent size-[12px] cursor-pointer group-hover/true:border-indigo-600 checked:bg-indigo-700"
                                    onClick={(_) => toggleBypassCertVerification(true)} />
                                <label for="bypass-true" class="cursor-pointer group-hover/true:text-white peer-checked:text-neutral-300">true</label>
                            </div>
                            <div class="group/false flex flex-row items-center space-x-1">
                                <input type="radio" name="bypass-cert-verification" id="bypass-false" value="false" checked={!config()?.settings?.bypassCertVerification}
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
                                <input type="radio" name="continue-on-faikure" id="continue-true" value="true" checked={config()?.settings?.continueOnFailure}
                                    class="peer text-indigo-600 bg-transparent size-[12px] cursor-pointer group-hover/true:border-indigo-600 checked:bg-indigo-700"
                                    onClick={(_) => toggleContinueOnFailure(true)} />
                                <label for="continue-true" class="cursor-pointer group-hover/true:text-white peer-checked:text-neutral-300">true</label>
                            </div>
                            <div class="group/false flex flex-row items-center space-x-1">
                                <input type="radio" name="continue-on-failure" id="continue-false" value="false" checked={!config()?.settings?.continueOnFailure}
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
                                value={config()?.settings?.environment || ""}
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
                                    value={global[0]}
                                    onInput={(_) => onGlobalInput(index())}
                                    onChange={(e) => onGlobalChange(index(), [e.currentTarget.value, global[1]])}
                                    class="flex-1 text-sm bg-transparent pl-2 p-1 border-1 border-b-0 group-last:border-b border-neutral-700 placeholder:text-neutral-500 focus-within:ring-2 focus-within:ring-inset focus-within:ring-indigo-600"
                                />
                                <input placeholder="Value"
                                    spellcheck={false}
                                    autocorrect="off"
                                    value={global[1]}
                                    onInput={(_) => onGlobalInput(index())}
                                    onChange={(e) => onGlobalChange(index(), [global[0], e.currentTarget.value])}
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
