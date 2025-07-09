import { createSignal, For } from 'solid-js';
import { $editorState } from '../../../stores/editorState';
import { EntityType } from '../../../stores/enum';

export const ConfigView = () => {

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile];
    let currentConfigFile = currentFile.type === EntityType.Config ? editorState.configFiles[currentFile.index] : undefined;

    let [config, setConfig] = createSignal(currentConfigFile);

    $editorState.subscribe((state) => {
        let currentFile = state.files[state.currentFile];
        let currentConfigFile = currentFile.type === EntityType.Config ? state.configFiles[currentFile.index] : undefined;
        setConfig(currentConfigFile);
    });

    return (
        <div class="p-3 flex flex-auto flex-col">
            <div class="flex flex-col">
                <div class="text-neutral-400 mt-2 mb-3 text-sm font-medium">Configurations</div>
                <ul class="w-full">
                    <li class="grid grid-cols-5 group text-neutral-300">
                        <div class="col-span-2 text-sm bg-transparent pl-2 p-1 border border-1 border-b-0 border-neutral-700">
                            API Key
                        </div>
                        <div class="col-span-3 text-sm bg-transparent pl-2 p-1 border border-1 border-l-0 border-b-0 border-neutral-700">
                            {config()?.apiKey ?? ""}
                        </div>
                    </li>
                    <li class="grid grid-cols-5 group text-neutral-300">
                        <div class="col-span-2 text-sm bg-transparent pl-2 p-1 border border-1 border-b-0 border-neutral-700">
                            Bypass Certificate Verification
                        </div>
                        <div class="col-span-3 text-sm bg-transparent pl-2 p-1 border border-1 border-l-0 border-b-0 border-neutral-700">
                            {config()?.bypassCertVerification ? "true" : "false"}
                        </div>
                    </li>
                    <li class="grid grid-cols-5 group text-neutral-300">
                        <div class="col-span-2 text-sm bg-transparent pl-2 p-1 border border-1 border-b-0 border-neutral-700">
                            Continue on Failure
                        </div>
                        <div class="col-span-3 text-sm bg-transparent pl-2 p-1 border border-1 border-l-0 border-b-0 border-neutral-700">
                            {config()?.continueOnFailure ? "true" : "false"}
                        </div>
                    </li>
                    <li class="grid grid-cols-5 group text-neutral-300">
                        <div class="col-span-2 text-sm bg-transparent pl-2 p-1 border border-1 border-neutral-700">
                            Environment
                        </div>
                        <div class="col-span-3 text-sm bg-transparent pl-2 p-1 border border-1 border-l-0 border-neutral-700">
                            {config()?.environment ?? ""}
                        </div>
                    </li>
                </ul>
            </div>
            <div class="flex flex-col mt-10">
                <div class="text-neutral-400 mb-3 text-sm font-medium">Global Variables</div>
                <ul class="w-full">
                    <For each={config()?.globals}>
                        {(global) => (
                            <li class="grid grid-cols-5 group text-neutral-300">
                                <div class="col-span-2 text-sm bg-transparent pl-2 p-1 border border-1 border-b-0 group-last:border-b border-neutral-700">
                                    {global.key}
                                </div>
                                <div class="col-span-3 text-sm bg-transparent pl-2 p-1 border border-1 border-l-0 border-b-0 group-last:border-b border-neutral-700">
                                    {global.value}
                                </div>
                            </li>
                        )}
                    </For>
                    <li class="grid grid-cols-5 group text-neutral-500">
                        <div class="col-span-2 text-sm bg-transparent pl-2 p-1 border border-1 border-neutral-700">
                            Key
                        </div>
                        <div class="col-span-3 text-sm bg-transparent pl-2 p-1 border border-1 border-l-0 border-neutral-700">
                            Value
                        </div>
                    </li>
                </ul>
            </div>
        </div>
    );
}
