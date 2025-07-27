import { createSignal } from 'solid-js';
import YAML from 'js-yaml';
import { $editorState, updateTestFile, type TestFile } from '../../../stores/editorState';
import MonacoEditorSolid from './MonacoEditorSolid';
import { $layoutState } from '../../../stores/layoutState';
import { EntityType, ViewMode } from '../../../stores/enum';
import { useStore } from '@nanostores/solid';

export const YamlView = () => {

    const pruneProperties = (key: string, value: string) => {
        return key === "generated" ? undefined : value
    }

    // Custom serializer that keeps body fields as JSON
    const serializeWithJsonBodies = (testFile: any) => {
        // Deep clone to avoid modifying original
        const cloned = JSON.parse(JSON.stringify(testFile, pruneProperties));
        
        // Extract body fields and replace with placeholders
        const bodyFields: { [key: string]: any } = {};
        let bodyCounter = 0;
        
        const extractBodies = (obj: any, path: string = ''): any => {
            if (obj === null || obj === undefined) return obj;
            
            if (typeof obj === 'object' && !Array.isArray(obj)) {
                const result: any = {};
                for (const [key, value] of Object.entries(obj)) {
                    if (key === 'body' && value !== null && value !== undefined) {
                        // Store the body content and replace with placeholder
                        const placeholder = `__JSON_BODY_${bodyCounter}__`;
                        bodyFields[placeholder] = value;
                        result[key] = placeholder;
                        bodyCounter++;
                    } else {
                        result[key] = extractBodies(value, path ? `${path}.${key}` : key);
                    }
                }
                return result;
            } else if (Array.isArray(obj)) {
                return obj.map((item, index) => extractBodies(item, `${path}[${index}]`));
            }
            
            return obj;
        };
        
        const processedFile = extractBodies(cloned);
        
        // Convert to YAML
        let yamlString = YAML.dump(processedFile);
        
        // Replace placeholders with formatted JSON
        for (const [placeholder, bodyContent] of Object.entries(bodyFields)) {
            const jsonString = JSON.stringify(bodyContent, null, 2);
            // Indent the JSON to match YAML indentation
            const indentedJson = jsonString.split('\n').map((line, index) => {
                if (index === 0) return line; // Don't indent first line
                return '    ' + line; // Indent subsequent lines
            }).join('\n');
            
            // Replace the placeholder with the indented JSON
            yamlString = yamlString.replace(`'${placeholder}'`, indentedJson);
            yamlString = yamlString.replace(`"${placeholder}"`, indentedJson);
            yamlString = yamlString.replace(placeholder, indentedJson);
        }
        
        return yamlString;
    };

    // Custom parser that handles JSON bodies in YAML
    const parseWithJsonBodies = (yamlString: string) => {
        // First, extract JSON bodies and replace with placeholders
        const bodyFields: { [key: string]: any } = {};
        let bodyCounter = 0;
        
        // Find JSON body sections (look for 'body:' followed by JSON)
        let processedYaml = yamlString;
        
        // Regex to find body: followed by JSON content
        const bodyRegex = /^(\s*)body:\s*(\{[\s\S]*?^\1(?=\S))/gm;
        
        processedYaml = processedYaml.replace(bodyRegex, (match, indent, jsonContent) => {
            try {
                // Try to parse the JSON content
                const parsed = JSON.parse(jsonContent.trim());
                const placeholder = `__JSON_BODY_${bodyCounter}__`;
                bodyFields[placeholder] = parsed;
                bodyCounter++;
                return `${indent}body: "${placeholder}"`;
            } catch {
                // If it's not valid JSON, leave it as is
                return match;
            }
        });
        
        // Parse the YAML
        const parsed = YAML.load(processedYaml);
        
        // Restore JSON bodies
        const restoreBodies = (obj: any): any => {
            if (obj === null || obj === undefined) return obj;
            
            if (typeof obj === 'object' && !Array.isArray(obj)) {
                const result: any = {};
                for (const [key, value] of Object.entries(obj)) {
                    if (key === 'body' && typeof value === 'string' && value.startsWith('__JSON_BODY_')) {
                        result[key] = bodyFields[value] || value;
                    } else {
                        result[key] = restoreBodies(value);
                    }
                }
                return result;
            } else if (Array.isArray(obj)) {
                return obj.map(item => restoreBodies(item));
            }
            
            return obj;
        };
        
        return restoreBodies(parsed);
    };

    const editorState = useStore($editorState);
    let currentFile = editorState().files[editorState().currentFile];
    let currentTestFile = currentFile.type === EntityType.Test ? editorState().testFiles[currentFile.index] : undefined;

    let [data, setData] = createSignal(currentTestFile ? serializeWithJsonBodies(currentTestFile.testFile) : "");

    $editorState.subscribe((state) => {
        let file = state.files[state.currentFile];
        let testFile = file.type === EntityType.Test ? state.testFiles[file.index] : undefined;

        // only update the data signal if the file changes
        if (file.id !== currentFile.id) {
            currentFile = file;
            currentTestFile = testFile;
            setData(testFile ? serializeWithJsonBodies(testFile.testFile) : "");
        }
    });

    $layoutState.subscribe((state, _, changedKey) => {
        if (changedKey !== "viewMode") return;

        // update the data when we go to view it
        if (state.viewMode === ViewMode.RAW) {
            setData(currentTestFile ? serializeWithJsonBodies(currentTestFile.testFile) : "");
        }
    });

    const onDataChange = (value: string) => {
        try {
            let updatedFile = parseWithJsonBodies(value) as TestFile;
            if (updatedFile.request) {
                updatedFile.request!.headers = updatedFile.request!.headers?.filter((h) => h !== null);
                if ((updatedFile.request!.headers?.length ?? 0) === 0) updatedFile.request!.headers = undefined;
                updatedFile.request!.params = updatedFile.request!.params?.filter((p) => p !== null);
                if ((updatedFile.request!.params?.length ?? 0) === 0) updatedFile.request!.params = undefined;
            }
            updateTestFile(updatedFile);
        } catch (error) {
            console.log("invalid yaml/json, skipping update:", error);
        }
    };

    return (
        <div class="flex flex-auto p-5">
            <MonacoEditorSolid value={data()} language="yaml" onChange={onDataChange} />
        </div>
    );
}
