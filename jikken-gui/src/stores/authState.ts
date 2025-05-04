import type { HttpHeader } from "./editorState";

const AUTH_TYPE_PREFIXES = new Set(["basic", "bearer"]);

export enum AuthType {
    None,
    Basic,
    Bearer,
};

const NO_AUTH = {
    type: AuthType.None,
};

export type BasicAuthData = {
    username?: string,
    password?: string,
};

export type BearerAuthData = {
    token?: string,
};

export type AuthState = {
    type: AuthType,
    data?: BasicAuthData | BearerAuthData,
};

export const parseAuthData = (headers?: HttpHeader[]) => {
    let authHeader = headers?.find((h) => h.header.toLocaleLowerCase() === "authorization") ?? undefined;
    if (!authHeader) return NO_AUTH;

    let parts = authHeader.value.split(" ");
    let authType = parts[0].toLocaleLowerCase();
    if (!AUTH_TYPE_PREFIXES.has(authType)) return NO_AUTH;

    if (authType === "bearer") {
        return {
            type: AuthType.Bearer,
            data: {
                token: parts[1]
            }
        }
    }

    let basicAuth = parseBasicAuthHeader(authHeader);
    return {
        type: AuthType.Basic,
        data: {
            username: basicAuth[0],
            password: basicAuth[1],
        },
    };
};

export const parseBasicAuthHeader = (header: HttpHeader) => {
    let parts = header.value.split(" ");
    if (parts.length < 2) return [undefined, undefined];
    try {
        let decoded = atob(parts[1]);
        let decodedParts = decoded.split(":");
        return [decodedParts[0], decodedParts.length > 1 ? decodedParts[1] : undefined];
    } catch {
        return [undefined, undefined];
    }
}
