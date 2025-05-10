use reqwest::{
    Body, Client, Method,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, error::Error, path::Path};
use std::{fs, time::SystemTime};
use tauri::{WebviewUrl, WebviewWindowBuilder};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderEntity {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub entities: Vec<FolderEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TestFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterate: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<Request>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup: Option<Setup>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare: Option<Compare>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Response>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stages: Option<Vec<Stage>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanup: Option<Cleanup>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<Variable>>,

    #[serde(skip_serializing, skip_deserializing)]
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Request {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<HttpVerb>,
    pub url: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<Parameter>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<Header>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>, // TODO: change this to json
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Compare {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<HttpVerb>,
    pub url: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<Parameter>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_params: Option<Vec<Parameter>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_params: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<Header>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_headers: Option<Vec<Header>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_headers: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<Header>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_schema: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract: Option<Vec<Extract>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    pub request: Request,
    pub response: Option<Response>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub request: Request,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare: Option<Compare>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Response>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<Variable>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Cleanup {
    pub onsuccess: Option<Request>,
    pub onfailure: Option<Request>,
    pub always: Option<Request>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum Variable {
    File(FileVariable),
    // Datum(UnvalidatedDatumSchemaVariable), // TODO
    Simple(SimpleValueVariable),
    ValueSet(ValueSetVariable),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Parameter {
    pub param: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Header {
    pub header: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpVerb {
    #[serde(alias = "get", alias = "GET")]
    Get,

    #[serde(alias = "post", alias = "POST")]
    Post,

    #[serde(alias = "put", alias = "PUT")]
    Put,

    #[serde(alias = "patch", alias = "PATCH")]
    Patch,

    #[serde(alias = "delete", alias = "DELETE")]
    Delete,

    Undefined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(deny_unknown_fields)]
pub struct Extract {
    pub name: String,
    pub field: String,
}

#[derive(Serialize, Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValueSetVariable {
    pub name: String,
    pub value_set: Vec<serde_json::Value>,
}

#[derive(Serialize, Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileVariable {
    pub name: String,
    pub file: String,
}

#[derive(Serialize, Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SimpleValueVariable {
    pub name: String,
    pub value: serde_json::Value,
}

#[derive(Serialize)]
pub struct HttpRequestResponse {
    pub status: u16,
    pub time: u128,
    pub headers: Vec<Header>,
    pub body: Option<String>,
}

#[tauri::command]
async fn open_folder_dialog(app: tauri::AppHandle) -> Option<FolderEntity> {
    use tauri_plugin_dialog::DialogExt;

    let folder_path = app.dialog().file().blocking_pick_folder();
    println!("folder: {:?}", folder_path);

    if let Some(dir) = folder_path {
        return open_folder(dir.as_path().unwrap());
    }

    None
}

#[tauri::command]
async fn open_folder_path(path: String) -> Option<FolderEntity> {
    let folder_path = Path::new(&path);
    let exists = fs::exists(folder_path);

    if exists.is_ok() {
        return open_folder(folder_path);
    }

    None
}

pub fn open_folder(path: &Path) -> Option<FolderEntity> {
    let mut entities = Vec::new();

    for entry in fs::read_dir(path.to_string_lossy().to_string()).unwrap() {
        let entry = entry.unwrap();
        let file_name = entry.file_name().to_string_lossy().to_string();
        println!("entry: {:?}", file_name);

        let path = entry.path();
        if path.is_dir() {
            entities.push(FolderEntity {
                name: file_name.clone(),
                path: path.to_string_lossy().to_string(),
                is_directory: true,
                entities: Vec::new(),
            });
        }

        if !file_name.ends_with(".jkt") {
            continue;
        }

        entities.push(FolderEntity {
            name: file_name,
            path: path.to_string_lossy().to_string(),
            is_directory: false,
            entities: Vec::new(),
        });
    }

    entities.sort_by(|a, b| a.name.cmp(&b.name));
    entities.sort_by(|a, b| a.is_directory.cmp(&b.is_directory));
    return Some(FolderEntity {
        name: path.file_name().unwrap().to_string_lossy().to_string(),
        path: path.to_string_lossy().to_string(),
        is_directory: true,
        entities,
    });
}

pub fn load(filename: &str) -> Result<TestFile, Box<dyn Error + Send + Sync>> {
    let file_data = fs::read_to_string(filename)?;
    let result: Result<TestFile, serde_yaml::Error> = serde_yaml::from_str(&file_data);
    match result {
        Ok(mut file) => {
            file.filename = String::from(filename);
            Ok(file)
        }
        Err(e) => {
            println!("unable to parse file ({}) data: {}", filename, e);
            Err(Box::from(e))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    pub name: String,
    pub path: String,
}

#[tauri::command]
async fn open_file(file: FileMetadata) -> Option<TestFile> {
    println!("received open_file() call: {:?}", file);

    match load(&file.path) {
        Ok(f) => Some(f),
        Err(e) => {
            println!("failed to open file: {}", e);
            None
        }
    }
}

#[tauri::command]
async fn save_new_file(app: tauri::AppHandle, test_file: TestFile) -> Option<FileMetadata> {
    use tauri_plugin_dialog::DialogExt;

    let Some(file_path) = app
        .dialog()
        .file()
        .add_filter("Jikken File", &["jkt"])
        .blocking_save_file()
    else {
        println!("failed to select path to save file");
        return None;
    };
    println!("selected file path: {:?}", file_path);

    let Some(path) = file_path.as_path() else {
        println!("failed to convert file path to path");
        return None;
    };

    let Ok(file_data) = serde_yaml::to_string(&test_file) else {
        println!("failed to serialize test file to YAML");
        return None;
    };

    match fs::write(file_path.as_path().unwrap(), file_data.as_bytes()) {
        Ok(_) => {
            println!("successfully saved file at path {}", file_path);
            Some(FileMetadata {
                name: path.file_name().unwrap().to_str().unwrap().to_string(),
                path: path.to_str().unwrap().to_string(),
            })
        }
        Err(error) => {
            println!("failed to write file {}", error);
            None
        }
    }
}

#[tauri::command]
async fn save_existing_file(file: FileMetadata, test_file: TestFile) -> Option<FileMetadata> {
    let path = Path::new(&file.path);

    let Ok(file_data) = serde_yaml::to_string(&test_file) else {
        println!("failed to serialize test file to YAML");
        return None;
    };

    match fs::write(path, file_data.as_bytes()) {
        Ok(_) => {
            println!("successfully saved file at path {}", file.path);
            Some(FileMetadata {
                name: path.file_name().unwrap().to_str().unwrap().to_string(),
                path: path.to_str().unwrap().to_string(),
            })
        }
        Err(error) => {
            println!("failed to write file {}", error);
            None
        }
    }
}

#[tauri::command]
async fn make_request(test_file: TestFile) -> Option<HttpRequestResponse> {
    let request = test_file.request.unwrap();
    let method = match request.method.unwrap_or(HttpVerb::Get) {
        HttpVerb::Get => Method::GET,
        HttpVerb::Post => Method::POST,
        HttpVerb::Put => Method::PUT,
        HttpVerb::Patch => Method::PATCH,
        HttpVerb::Delete => Method::DELETE,
        _ => Method::GET,
    };

    let mut headers = HeaderMap::new();
    request
        .headers
        .unwrap_or_default()
        .into_iter()
        .for_each(|h| {
            let head: Cow<'static, str> = h.header.into();
            headers.insert(
                HeaderName::from_bytes(head.as_bytes()).unwrap(),
                HeaderValue::from_str(h.value.as_str()).unwrap(),
            );
        });

    let mut params: Vec<(String, String)> = Vec::new();
    request.params.unwrap_or_default().iter().for_each(|p| {
        params.push((p.param.clone(), p.value.clone()));
    });

    let client = Client::new();

    let timer = SystemTime::now();
    let mut client_request = client
        .request(method, request.url)
        .headers(headers)
        .query(&params);
    if let Some(body) = request.body {
        client_request = client_request.body(Body::from(serde_json::to_string(&body).unwrap()));
    }

    match client_request.send().await {
        Ok(response) => {
            let end = timer.elapsed().unwrap();
            let headers = response
                .headers()
                .iter()
                .map(|h| Header {
                    header: h.0.to_string(),
                    value: h.1.to_str().unwrap().to_string(),
                })
                .collect();

            Some(HttpRequestResponse {
                status: response.status().as_u16(),
                time: end.as_millis(),
                headers,
                body: Some(response.text().await.unwrap()),
            })
        }
        Err(_) => None,
    }
}

#[tauri::command]
async fn save_response_body(app: tauri::AppHandle, body: String) -> Option<FileMetadata> {
    use tauri_plugin_dialog::DialogExt;

    let Some(file_path) = app
        .dialog()
        .file()
        .add_filter("JSON", &["json"])
        .blocking_save_file()
    else {
        println!("failed to select path to save file");
        return None;
    };
    println!("selected file path: {:?}", file_path);

    let Some(path) = file_path.as_path() else {
        println!("failed to convert file path to path");
        return None;
    };

    match fs::write(file_path.as_path().unwrap(), body.as_bytes()) {
        Ok(_) => {
            println!("successfully saved file at path {}", file_path);
            Some(FileMetadata {
                name: path.file_name().unwrap().to_str().unwrap().to_string(),
                path: path.to_str().unwrap().to_string(),
            })
        }
        Err(error) => {
            println!("failed to write file {}", error);
            None
        }
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Jikken")
                .inner_size(1200.0, 800.0);

            // set transparent title bar only when building for macOS
            #[cfg(target_os = "macos")]
            let win_builder = win_builder.title_bar_style(tauri::TitleBarStyle::Transparent);

            let window = win_builder.build().unwrap();

            // set background color only when building for macOS
            #[cfg(target_os = "macos")]
            {
                use cocoa::appkit::{NSColor, NSWindow};
                use cocoa::base::{id, nil};

                let ns_window = window.ns_window().unwrap() as id;
                unsafe {
                    let bg_color = NSColor::colorWithRed_green_blue_alpha_(
                        nil,
                        38.0 / 255.0,
                        38.0 / 255.0,
                        38.0 / 255.0,
                        1.0,
                    );
                    ns_window.setBackgroundColor_(bg_color);
                }
            }

            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            open_folder_path,
            open_folder_dialog,
            open_file,
            save_new_file,
            save_existing_file,
            make_request,
            save_response_body,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
