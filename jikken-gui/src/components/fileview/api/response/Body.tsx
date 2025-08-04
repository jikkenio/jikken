import { createSignal, Show } from 'solid-js';
import { $editorState, saveResponseBody } from '../../../../stores/editorState';
import MonacoEditorSolid from '../MonacoEditorSolid';
import { NotificationType, triggerNotification } from '../../../../stores/notificationState';
import { EntityType } from '../../../../stores/enum';
import { tippy } from '../../../TippySolid';

export const Body = () => {

    tippy

    enum BodyType {
        None,
        Json,
        Html,
        Xml,
        Text,
    };

    const prettyPrintHtml = (html: string): string => {
        try {
            const parser = new DOMParser();
            const doc = parser.parseFromString(html, 'text/html');

            // Check for parsing errors
            const errorNode = doc.querySelector('parsererror');
            if (errorNode) {
                return html; // Return original if parsing failed
            }

            return formatElement(doc.documentElement, 0);
        } catch {
            return html; // Return original if any error occurs
        }
    };

    const prettyPrintXml = (xml: string): string => {
        try {
            const parser = new DOMParser();
            const doc = parser.parseFromString(xml, 'application/xml');

            // Check for parsing errors
            const errorNode = doc.querySelector('parsererror');
            if (errorNode) {
                return xml; // Return original if parsing failed
            }

            return formatElement(doc.documentElement, 0);
        } catch {
            return xml; // Return original if any error occurs
        }
    };

    const formatElement = (element: Element, depth: number): string => {
        const indent = '  '.repeat(depth);

        let result = `${indent}<${element.tagName.toLowerCase()}`;

        // Add attributes
        for (let i = 0; i < element.attributes.length; i++) {
            const attr = element.attributes[i];
            result += ` ${attr.name}="${attr.value}"`;
        }

        if (element.children.length === 0 && !element.textContent?.trim()) {
            // Self-closing tag
            result += ' />';
            return result;
        }

        result += '>';

        // Handle text content
        const textContent = element.textContent?.trim();
        const hasElementChildren = element.children.length > 0;

        if (hasElementChildren) {
            result += '\n';
            // Add child elements
            for (let i = 0; i < element.children.length; i++) {
                result += formatElement(element.children[i], depth + 1);
                if (i < element.children.length - 1) {
                    result += '\n';
                }
            }
            result += `\n${indent}`;
        } else if (textContent) {
            // Pure text content
            result += textContent;
        }

        result += `</${element.tagName.toLowerCase()}>`;
        return result;
    };

    type Body = {
        type: BodyType,
        content?: string,
        language?: string,
    };

    const pretty = (body: string | undefined, bodyType: BodyType) => {
        if (!body) return undefined;

        switch (bodyType) {
            case BodyType.Json:
                try {
                    return JSON.stringify(JSON.parse(body), null, 2);
                } catch {
                    // If JSON parsing fails, return as-is
                    return body;
                }
            case BodyType.Html:
                return prettyPrintHtml(body);
            case BodyType.Xml:
                return prettyPrintXml(body);
            default:
                // For Text and other types, return as-is
                return body;
        }
    }

    const getContentType = (headers: any[] | undefined): string | undefined => {
        if (!headers) return undefined;
        const contentTypeHeader = headers.find(h => h.header.toLowerCase() === 'content-type');
        return contentTypeHeader?.value?.toLowerCase();
    }

    const detectBodyType = (contentType: string | undefined, body: string | undefined): { type: BodyType, language: string } => {
        if (!body) return { type: BodyType.None, language: 'text' };

        if (!contentType) {
            // Try to detect based on content
            const trimmed = body.trim();
            if (trimmed.startsWith('{') || trimmed.startsWith('[')) {
                return { type: BodyType.Json, language: 'json' };
            } else if (trimmed.startsWith('<!DOCTYPE') || trimmed.startsWith('<html')) {
                return { type: BodyType.Html, language: 'html' };
            } else if (trimmed.startsWith('<?xml') || trimmed.startsWith('<')) {
                return { type: BodyType.Xml, language: 'xml' };
            }
            return { type: BodyType.Text, language: 'text' };
        }

        if (contentType.includes('json')) {
            return { type: BodyType.Json, language: 'json' };
        } else if (contentType.includes('html')) {
            return { type: BodyType.Html, language: 'html' };
        } else if (contentType.includes('xml')) {
            return { type: BodyType.Xml, language: 'xml' };
        } else {
            return { type: BodyType.Text, language: 'text' };
        }
    }

    let editorState = $editorState.get();
    let currentFile = editorState.files[editorState.currentFile];
    let currentTestFile = currentFile.type === EntityType.Test ? editorState.testFiles[currentFile.index] : undefined;
    let currentResponse = currentTestFile?.response;
    let contentType = getContentType(currentResponse?.headers);
    let bodyInfo = detectBodyType(contentType, currentResponse?.body);

    const [body, setBody] = createSignal({
        type: bodyInfo.type,
        content: pretty(currentResponse?.body, bodyInfo.type),
        language: bodyInfo.language
    } as Body)

    $editorState.subscribe((state) => {
        currentFile = state.files[state.currentFile];
        currentTestFile = currentFile.type === EntityType.Test ? state.testFiles[currentFile.index] : undefined;
        const response = currentTestFile?.response;
        const contentType = getContentType(response?.headers);
        const bodyInfo = detectBodyType(contentType, response?.body);
        setBody({
            type: bodyInfo.type,
            content: pretty(response?.body, bodyInfo.type),
            language: bodyInfo.language
        } as Body);
    });

    const copy = async () => {
        console.log("copying response body to clipboard");
        if (!navigator.clipboard) {
            console.log("no clipboard support found");
            return;
        }

        let currentBody = body();
        if (currentBody.type === BodyType.None) {
            console.log("no body found");
            return;
        }

        await navigator.clipboard.writeText(currentBody.content!);
        console.log("successfully copied to clipboard");
        triggerNotification(NotificationType.Success, "Successfully copied to clipboard!");
    };

    return (
        <div class="p-3 pt-0 flex flex-auto h-full overflow-hidden min-w-0">
            <div class="group flex flex-col flex-auto h-full min-w-0">
                <Show when={body().content}>
                    <div class="flex w-full justify-end pointer-events-none">
                        <div class="absolute z-10 mt-px mr-px flex flex-row pointer-events-auto cursor-pointer text-neutral-500 bg-transparent invisible group-hover:visible">
                            <span class="p-2 pr-1.5 hover:text-neutral-200"
                                onClick={copy}
                                use:tippy={{
                                    props: {
                                        content: "Copy"
                                    }
                                }}>
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
                                onClick={saveResponseBody}
                                use:tippy={{
                                    props: {
                                        content: "Save to file"
                                    }
                                }}>
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
                    <div class="w-full flex-auto h-full min-w-0">
                        <MonacoEditorSolid value={body().content} language={body().language || 'text'} readonly />
                    </div>
                </Show>
                <Show when={currentTestFile?.response?.status && !body().content}>
                    <div class="flex flex-auto justify-center items-center">
                        <span class="text-sm text-neutral-600">This response has no body.</span>
                    </div>
                </Show>
                <Show when={currentTestFile?.executing}>
                    <div class="flex flex-auto justify-center items-center">
                        <span class="text-sm text-neutral-600">Executing request...</span>
                    </div>
                </Show>
            </div>
        </div>
    );
}
