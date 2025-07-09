export enum EntityType {
    Test,
    Config,
    Directory
};

export enum AuthType {
    None,
    Basic,
    Bearer,
};

export enum HttpVerb {
    GET = "Get",
    POST = "Post",
    PUT = "Put",
    PATCH = "Patch",
    DELETE = "Delete",
};

export enum ViewMode {
    API,
    RAW,
    TEST,
};