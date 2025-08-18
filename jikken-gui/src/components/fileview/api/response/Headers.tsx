import { createSignal, For } from 'solid-js';
import { $editorState } from '../../../../stores/editorState';
import { EntityType } from '../../../../stores/enum';

export const Headers = (props: { compare: boolean }) => {

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile];
    let currentTestFile = currentFile.type === EntityType.Test ? editorState.testFiles[currentFile.index] : undefined;
    let currentResponse = currentTestFile?.responses;

    const [headers, setHeaders] = createSignal((props.compare ? currentResponse?.compare?.headers : currentResponse?.request?.headers) ?? []);

    $editorState.subscribe((state) => {
        let file = state.files[state.currentFile];
        let testFile = file.type === EntityType.Test ? state.testFiles[file.index] : undefined;
        setHeaders((props.compare ? testFile?.responses.compare?.headers : testFile?.responses.request?.headers) ?? []);
    });

    return (
        <div class="p-3 pt-0 flex flex-auto h-full overflow-hidden min-w-0 max-h-full min-h-0">
            <div class="group flex flex-col flex-auto h-full min-w-0 max-h-full min-h-0">
                <div class="text-neutral-400 mb-2 text-sm font-medium flex-shrink-0">Headers</div>
                <div class="flex-auto overflow-y-auto min-h-0 max-h-full">
                    <ul class="w-full">
                        <For each={headers()}>
                            {(header) => (
                                <li class="grid grid-cols-5 group text-neutral-300">
                                    <div class="col-span-2 text-sm bg-transparent pl-2 p-1 border border-1 border-b-0 group-last:border-b border-neutral-700 min-w-0 overflow-hidden text-ellipsis">
                                        {header.header}
                                    </div>
                                    <div class="col-span-3 text-sm bg-transparent pl-2 p-1 border border-1 border-l-0 border-b-0 group-last:border-b border-neutral-700 min-w-0 overflow-hidden text-ellipsis">
                                        {header.value}
                                    </div>
                                </li>
                            )}
                        </For>
                    </ul>
                </div>
            </div>
        </div>
    );
};
