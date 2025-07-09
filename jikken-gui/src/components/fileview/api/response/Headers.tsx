import { For } from 'solid-js';
import { $editorState } from '../../../../stores/editorState';
import { EntityType } from '../../../../stores/enum';
import { useStore } from '@nanostores/solid';

export const Headers = () => {

    const editorState = useStore($editorState);

    const currentFile = () => editorState().files[editorState().currentFile];
    const currentTestFile = () => currentFile().type === EntityType.Test ? editorState().testFiles[currentFile().index] : undefined;
    const headers = () => currentTestFile()?.response?.headers ?? [];

    return (
        <div class="p-3 pt-1">
            <div class="text-neutral-400 mb-2 text-sm font-medium">Headers</div>
            <ul class="w-full">
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
