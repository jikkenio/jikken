import { Show } from 'solid-js';
import { $editorState, saveResponseBody } from '../../../../stores/editorState';
import MonacoEditorSolid from '../MonacoEditorSolid';
import { NotificationType, triggerNotification } from '../../../../stores/notificationState';
import { EntityType } from '../../../../stores/enum';
import { useStore } from '@nanostores/solid';

export const Body = () => {

    enum BodyType {
        None,
        Json,
    };

    type Body = {
        type: BodyType,
        content?: string,
    };

    const editorState = useStore($editorState);

    const currentFile = () => editorState().files[editorState().currentFile];
    const currentTestFile = () => currentFile().type === EntityType.Test ? editorState().testFiles[currentFile().index] : undefined;
    const currentResponse = () => currentTestFile()?.testFile.response;
    const body = () => ({ type: currentResponse() ? BodyType.Json : BodyType.None, content: currentResponse()?.body } as Body);

    const copy = async () => {
        console.log("copying response body to clipboard");
        if (!navigator.clipboard) {
            console.log("no clipboard support found");
            return;
        }

        let currentBody = body();
        if (currentBody.type !== BodyType.Json) {
            console.log("no body found");
            return;
        }

        await navigator.clipboard.writeText(currentBody.content!);
        console.log("successfully copied to clipboard");
        triggerNotification(NotificationType.Success, "Successfully copied to clipboard!");
    };

    return (
        <div class="p-3 pt-0 flex flex-auto">
            <div class="group flex flex-col flex-auto">
                <Show when={body().content}>
                    <div class="flex w-full justify-end pointer-events-none">
                        <div class="absolute z-10 mt-px mr-px flex flex-row pointer-events-auto cursor-pointer text-neutral-500 bg-neutral-800 invisible group-hover:visible">
                            <span class="p-2 pr-1.5 hover:text-neutral-200"
                                onClick={() => copy()}>
                                <svg xmlns="http://www.w3.org/2000/svg"
                                    width="16"
                                    height="16"
                                    fill="currentColor"
                                    class="bi bi-copy"
                                    viewBox="0 0 16 16">
                                    <path fill-rule="evenodd" d="M4 2a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2zm2-1a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V2a1 1 0 0 0-1-1zM2 5a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1v-1h1v1a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h1v1z" />
                                </svg>
                            </span>
                            <span class="p-2 pl-1.5 hover:text-neutral-200"
                                onClick={() => saveResponseBody()}>
                                <svg xmlns="http://www.w3.org/2000/svg"
                                    width="16"
                                    height="16"
                                    fill="currentColor"
                                    class="bi bi-floppy"
                                    viewBox="0 0 16 16">
                                    <path d="M11 2H9v3h2z" />
                                    <path d="M1.5 0h11.586a1.5 1.5 0 0 1 1.06.44l1.415 1.414A1.5 1.5 0 0 1 16 2.914V14.5a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 0 14.5v-13A1.5 1.5 0 0 1 1.5 0M1 1.5v13a.5.5 0 0 0 .5.5H2v-4.5A1.5 1.5 0 0 1 3.5 9h9a1.5 1.5 0 0 1 1.5 1.5V15h.5a.5.5 0 0 0 .5-.5V2.914a.5.5 0 0 0-.146-.353l-1.415-1.415A.5.5 0 0 0 13.086 1H13v4.5A1.5 1.5 0 0 1 11.5 7h-7A1.5 1.5 0 0 1 3 5.5V1H1.5a.5.5 0 0 0-.5.5m3 4a.5.5 0 0 0 .5.5h7a.5.5 0 0 0 .5-.5V1H4zM3 15h10v-4.5a.5.5 0 0 0-.5-.5h-9a.5.5 0 0 0-.5.5z" />
                                </svg>
                            </span>
                        </div>
                    </div>
                    <div class="flex flex-auto w-full">
                        <MonacoEditorSolid value={body().content} language="json" readonly />
                    </div>
                </Show>
                <Show when={currentResponse()?.status && !body().content}>
                    <div class="flex flex-auto justify-center items-center">
                        <span class="text-sm text-neutral-600">This response has no body.</span>
                    </div>
                </Show>
            </div>
        </div>
    );
}
