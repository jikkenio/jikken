import { createSignal, For } from 'solid-js';
import { $editorState } from '../../../stores/editorState';

export const Headers = () => {

    let editorState = $editorState.get();
    let currentResponse = editorState.files[editorState.currentFile].response;
    let [headers, setHeaders] = createSignal(currentResponse?.headers ?? []);

    $editorState.subscribe((state) => {
        let response = state.files[state.currentFile].response;
        setHeaders(response?.headers ?? []);
        currentResponse = response;
    });

    return (
<<<<<<< HEAD
        <div class="p-3 pt-1">
            <div class="text-neutral-400 mb-2 text-sm font-medium">Headers</div>
            <ul class="w-full">
=======
        <div id="tab-headers-panel" class="p-3 pt-1">
            <div class="text-neutral-400 mb-2 text-sm font-medium">Headers</div>

            <ul>
>>>>>>> 330f865 (JK-594: initial move of jikken-gui project into repo)
                <For each={headers()}>
                    {(header) => (
                        <li class="grid grid-cols-5 group text-neutral-300">
                            <div class="col-span-2 text-sm bg-transparent pl-2 p-1 border border-1 border-b-0 group-last:border-b border-neutral-700">
                                {header.header}
                            </div>
                            <div class="col-span-3 text-sm bg-transparent pl-2 p-1 border border-1 border-l-0 border-b-0 group-last:border-b border-neutral-700">
                                {header.value}
                            </div>
                        </li>
                    )}
                </For>
            </ul>
        </div>
    );
}
