use super::errors::GenericError;
use super::test::template;
use log::{error, info};

use crate::test::http;
use crate::test::File;
use std::error::Error;
use std::io::Write;
use tokio::fs;
use tokio::io::AsyncWriteExt;

fn create_tags(tags: &[String]) -> Option<String> {
    if tags.is_empty() {
        None
    } else {
        Some(tags.join(","))
    }
}

fn create_status_code(status_code_pattern: &str) -> Option<u16> {
    if status_code_pattern == "2XX" {
        Some(200)
    } else {
        status_code_pattern.parse().ok()
    }
}

fn create_filename(path_string: &str, verb: &http::Verb) -> String {
    let mut path = path_string
        .split('/')
        .filter(|s| *s != "")
        .collect::<Vec<&str>>()
        .join(std::path::MAIN_SEPARATOR_STR);

    if path == "" {
        path = "ROOT".to_string();
    }

    std::path::PathBuf::from(path)
        .join(format!("{:?}.jkt", verb))
        .to_str()
        .unwrap()
        .to_string()
}

mod openapi_legacy {
    use super::*;
    use crate::test;
    use crate::test::file::UnvalidatedRequest;
    use crate::test::file::UnvalidatedResponse;
    use crate::test::File;
    use openapiv3::IndexMap;
    use openapiv3::{Operation, PathItem, RefOr, Responses, Server, VersionedOpenAPI};
    use std::collections::hash_map::RandomState;
    use std::io::BufReader;

    fn create_headers(
        headers: &IndexMap<String, RefOr<openapiv3::Header>, RandomState>,
    ) -> Option<Vec<test::http::Header>> {
        let ret: Vec<test::http::Header> = headers
            .iter()
            .map(|(name, _)| test::http::Header {
                header: name.clone(),
                matches_variable: std::cell::Cell::new(false),
                value: String::default(),
            })
            .collect();

        if !ret.is_empty() {
            Some(ret)
        } else {
            None
        }
    }

    fn create_response(responses: &Responses) -> Option<UnvalidatedResponse> {
        responses
            .responses
            .iter()
            .map(|(sc, obj_or_ref)| (sc.to_string(), obj_or_ref))
            .filter(|(status_code_pattern, _)| status_code_pattern.starts_with("2"))
            .map(|(status_code_pattern, obj_or_ref)| match obj_or_ref {
                RefOr::Item(t) => Some(UnvalidatedResponse {
                    status: create_status_code(status_code_pattern.as_str()),
                    body: None, //would need a way to validate against a provided schema
                    headers: create_headers(&t.headers),
                    extract: None,
                    ignore: None,
                }),
                _ => None,
            })
            .last()
            .flatten()
    }

    fn create_request(
        url: &str,
        verb: test::http::Verb,
        op: &openapiv3::Operation,
    ) -> UnvalidatedRequest {
        let mut headers: Vec<test::http::Header> = vec![];
        let mut parameters: Vec<test::http::Parameter> = vec![];

        op.parameters.iter().for_each(|f| match f {
            RefOr::Item(t) => {
                match &t.kind {
                    openapiv3::ParameterKind::Query { .. } => {
                        parameters.push(test::http::Parameter {
                            param: t.name.clone(),
                            value: String::default(),
                            matches_variable: std::cell::Cell::new(false),
                        })
                    }
                    openapiv3::ParameterKind::Header { .. } => headers.push(test::http::Header {
                        header: t.name.clone(),
                        value: String::default(),
                        matches_variable: std::cell::Cell::new(false),
                    }),
                    openapiv3::ParameterKind::Path { .. } => (), //user will have to do this themselves, based upon generated template
                    openapiv3::ParameterKind::Cookie { .. } => (), //no cookie support
                }
            }
            _ => (),
        });

        UnvalidatedRequest {
            body: None,
            method: Some(verb),
            url: url.to_string(),
            headers: if headers.is_empty() {
                None
            } else {
                Some(headers)
            },
            params: if parameters.is_empty() {
                None
            } else {
                Some(parameters)
            },
        }
    }

    pub fn get_test_paths(
        root_servers: &[Server],
        path_servers: &[Server],
        op_servers: &[Server],
        fallback: &str,
    ) -> Vec<String> {
        let url_extractor = |servers: &[Server]| -> Option<Vec<String>> {
            if servers.is_empty() {
                None
            } else {
                Some(servers.into_iter().map(|s| s.url.clone()).collect())
            }
        };

        url_extractor(path_servers)
            .or(url_extractor(root_servers))
            .or(url_extractor(op_servers))
            .unwrap_or(vec![fallback.to_string()])
    }

    fn create_tests_for_op(
        op: &Option<Operation>,
        root_servers: &[Server],
        path: &PathItem,
        path_string: &str,
        verb: test::http::Verb,
        full: bool,
        multistage: bool,
    ) -> Vec<File> {
        op.clone()
            .map(|op| {
                get_test_paths(root_servers, &path.servers, &op.servers, "{url}")
                    .iter()
                    .map(|url| {
                        create_test(
                            format!("{}{}", url, path_string).as_str(),
                            &op,
                            verb,
                            full,
                            multistage,
                            path_string,
                        )
                    })
                    .flatten()
                    .collect::<Vec<File>>()
            })
            .unwrap_or_default()
    }

    fn create_variables(op: &openapiv3::Operation) -> Option<Vec<test::file::UnvalidatedVariable>> {
        let ret = op
            .parameters
            .iter()
            .map(|p_or_ref| match p_or_ref {
                RefOr::Reference { .. } => None,
                RefOr::Item(t) => Some(test::file::UnvalidatedVariable {
                    name: t.name.clone(),
                    data_type: None,
                    file: None,
                    format: None,
                    modifier: None,
                    value: None,
                }),
            })
            .filter(Option::is_some)
            .collect::<Option<Vec<test::file::UnvalidatedVariable>>>()
            .unwrap_or_default();

        if ret.is_empty() {
            None
        } else {
            Some(ret)
        }
    }

    fn create_test(
        path: &str,
        op: &openapiv3::Operation,
        verb: test::http::Verb,
        full: bool,
        multistage: bool,
        path_string: &str,
    ) -> Option<File> {
        let default = if full {
            test::template::template_full().unwrap()
        } else if multistage {
            test::template::template_staged().unwrap()
        } else {
            test::File::default()
        };

        let resolved_path = path.replace("{", "${").to_string();
        let request = create_request(resolved_path.as_str(), verb, op);
        let response = create_response(&op.responses).or(Some(UnvalidatedResponse::default()));
        let variables = create_variables(&op);

        if multistage || verb == test::http::Verb::Delete {
            Some(File {
                name: op.summary.clone().or(default.name),
                id: op.operation_id.clone().or(default.id),
                tags: create_tags(&op.tags),
                stages: Some(vec![test::file::UnvalidatedStage {
                    request: request,
                    compare: None,
                    response: response,
                    variables: variables,
                    name: None,
                    delay: None,
                }]),
                filename: create_filename(path_string, &verb),
                ..default
            })
        } else {
            Some(File {
                name: op.summary.clone().or(default.name),
                id: op.operation_id.clone().or(default.id),
                tags: create_tags(&op.tags),
                response,
                request: Some(request),
                filename: create_filename(path_string, &verb),
                variables,
                ..default
            })
        }
    }

    fn create_tests(
        root_servers: &[Server],
        path_string: &str,
        path: &PathItem,
        full: bool,
        multistage: bool,
    ) -> Vec<File> {
        let stuff: [(&Option<Operation>, test::http::Verb); 5] = [
            (&path.get, test::http::Verb::Get),
            (&path.post, test::http::Verb::Post),
            (&path.delete, test::http::Verb::Delete),
            (&path.patch, test::http::Verb::Patch),
            (&path.put, test::http::Verb::Put),
        ];

        stuff
            .into_iter()
            .map(|(op, verb)| {
                create_tests_for_op(op, root_servers, path, path_string, verb, full, multistage)
            })
            .flatten()
            .collect()
    }

    pub fn create_tests_from_openapi_spec(
        file: &str,
        full: bool,
        multistage: bool,
    ) -> Result<Vec<File>, Box<dyn std::error::Error + Send + Sync>> {
        let file = std::fs::File::open(file)?;
        let reader = BufReader::new(file);

        let versioned_openapi: Result<VersionedOpenAPI, serde_json::Error> =
            serde_json::from_reader(reader);
        match versioned_openapi {
            Err(e) => Err(Box::from(e)),
            Ok(v) => {
                let openapi = v.upgrade();
                Ok(openapi
                    .paths
                    .iter()
                    .map(|(path_string, ref_or_path)| match ref_or_path {
                        RefOr::Item(path) => {
                            create_tests(&openapi.servers, path_string, path, full, multistage)
                        }
                        RefOr::Reference { .. } => Vec::default(),
                    })
                    .flatten()
                    .collect())
            }
        }
    }
}

mod openapi_v31 {
    use super::*;
    use crate::test;
    use crate::test::file::UnvalidatedRequest;
    use crate::test::file::UnvalidatedResponse;
    use crate::test::File;
    use oas3;
    use oas3::spec::Header;
    use oas3::spec::ObjectOrReference;
    use oas3::spec::Operation;
    use oas3::spec::PathItem;
    use oas3::spec::Response;
    use oas3::spec::Server;
    use std::collections::BTreeMap;
    pub fn get_test_paths(
        root_servers: &[Server],
        path_servers: &[Server],
        op_servers: &[Server],
        fallback: &str,
    ) -> Vec<String> {
        let url_extractor = |servers: &[Server]| -> Option<Vec<String>> {
            if servers.is_empty() {
                None
            } else {
                Some(servers.into_iter().map(|s| s.url.clone()).collect())
            }
        };

        url_extractor(path_servers)
            .or(url_extractor(root_servers))
            .or(url_extractor(op_servers))
            .unwrap_or(vec![fallback.to_string()])
    }

    fn create_headers(
        headers: &BTreeMap<String, ObjectOrReference<Header>>,
    ) -> Option<Vec<test::http::Header>> {
        let ret: Vec<test::http::Header> = headers
            .iter()
            .map(|(name, _)| test::http::Header {
                header: name.clone(),
                matches_variable: std::cell::Cell::new(false),
                value: String::default(),
            })
            .collect();

        if !ret.is_empty() {
            Some(ret)
        } else {
            None
        }
    }

    fn create_response(
        responses: &BTreeMap<String, ObjectOrReference<Response>>,
    ) -> Option<UnvalidatedResponse> {
        responses
            .iter()
            .filter(|(status_code_pattern, _)| status_code_pattern.starts_with("2"))
            .map(|(status_code_pattern, obj_or_ref)| match obj_or_ref {
                ObjectOrReference::Object(t) => Some(UnvalidatedResponse {
                    status: create_status_code(status_code_pattern),
                    body: None, //would need a way to validate against a provided schema
                    headers: create_headers(&t.headers),
                    extract: None,
                    ignore: None,
                }),
                _ => None,
            })
            .last()
            .flatten()
    }

    fn create_request(
        url: &str,
        verb: test::http::Verb,
        op: &oas3::spec::Operation,
    ) -> UnvalidatedRequest {
        let mut headers: Vec<test::http::Header> = vec![];
        let mut parameters: Vec<test::http::Parameter> = vec![];

        op.parameters.iter().for_each(|f| match f {
            ObjectOrReference::Object(t) => {
                match t.location.as_str() {
                    "query" => parameters.push(test::http::Parameter {
                        param: t.name.clone(),
                        value: String::default(),
                        matches_variable: std::cell::Cell::new(false),
                    }),
                    "header" => headers.push(test::http::Header {
                        header: t.name.clone(),
                        value: String::default(),
                        matches_variable: std::cell::Cell::new(false),
                    }),
                    "path" => (), //user will have to do this themselves, based upon generated template
                    "cookie" => (), //no cookie support
                    _ => (),
                }
            }
            ObjectOrReference::Ref { .. } => (),
        });

        UnvalidatedRequest {
            body: None,
            method: Some(verb),
            url: url.to_string(),
            headers: if headers.is_empty() {
                None
            } else {
                Some(headers)
            },
            params: if parameters.is_empty() {
                None
            } else {
                Some(parameters)
            },
        }
    }

    fn create_variables(op: &Operation) -> Option<Vec<test::file::UnvalidatedVariable>> {
        let ret = op
            .parameters
            .iter()
            .map(|p_or_ref| match p_or_ref {
                ObjectOrReference::Ref { .. } => None,
                ObjectOrReference::Object(t) => Some(test::file::UnvalidatedVariable {
                    name: t.name.clone(),
                    data_type: None,
                    file: None,
                    format: None,
                    modifier: None,
                    value: None,
                }),
            })
            .filter(Option::is_some)
            .collect::<Option<Vec<test::file::UnvalidatedVariable>>>()
            .unwrap_or_default();

        if ret.is_empty() {
            None
        } else {
            Some(ret)
        }
    }

    fn create_test(
        path: &str,
        op: &oas3::spec::Operation,
        verb: test::http::Verb,
        full: bool,
        multistage: bool,
        path_string: &str,
    ) -> Option<File> {
        let default = if full {
            test::template::template_full().unwrap()
        } else if multistage {
            test::template::template_staged().unwrap()
        } else {
            test::File::default()
        };

        let resolved_path = path.replace("{", "${").to_string();
        let request = create_request(resolved_path.as_str(), verb, op);
        let response = create_response(&op.responses).or(Some(UnvalidatedResponse::default()));
        let variables = create_variables(&op);

        if multistage || verb == test::http::Verb::Delete {
            Some(File {
                name: op.summary.clone().or(default.name),
                id: op.operation_id.clone().or(default.id),
                tags: create_tags(&op.tags),
                stages: Some(vec![test::file::UnvalidatedStage {
                    request: request,
                    compare: None,
                    response: response,
                    variables: variables,
                    name: None,
                    delay: None,
                }]),
                filename: create_filename(path_string, &verb),
                ..default
            })
        } else {
            Some(File {
                name: op.summary.clone().or(default.name),
                id: op.operation_id.clone().or(default.id),
                tags: create_tags(&op.tags),
                response,
                request: Some(request),
                filename: create_filename(path_string, &verb),
                variables,
                ..default
            })
        }
    }

    fn create_tests_for_op(
        op: &Option<Operation>,
        root_servers: &[Server],
        path: &PathItem,
        path_string: &str,
        verb: test::http::Verb,
        full: bool,
        multistage: bool,
    ) -> Vec<File> {
        op.clone()
            .map(|op| {
                get_test_paths(root_servers, &path.servers, &op.servers, "${url}")
                    .into_iter()
                    .map(|url| {
                        create_test(
                            format!("{}{}", url, path_string).as_str(),
                            &op,
                            verb,
                            full,
                            multistage,
                            path_string,
                        )
                    })
                    .flatten()
                    .collect::<Vec<File>>()
            })
            .unwrap_or_default()
    }

    fn create_tests(
        root_servers: &[Server],
        path_string: &str,
        path: &PathItem,
        full: bool,
        multistage: bool,
    ) -> Vec<File> {
        let stuff: [(&Option<Operation>, test::http::Verb); 5] = [
            (&path.get, test::http::Verb::Get),
            (&path.post, test::http::Verb::Post),
            (&path.delete, test::http::Verb::Delete),
            (&path.patch, test::http::Verb::Patch),
            (&path.put, test::http::Verb::Put),
        ];

        stuff
            .into_iter()
            .map(|(op, verb)| {
                create_tests_for_op(op, root_servers, path, path_string, verb, full, multistage)
            })
            .flatten()
            .collect()
    }

    pub fn create_tests_from_openapi_spec(
        file: &str,
        full: bool,
        multistage: bool,
    ) -> Result<Vec<File>, Box<dyn std::error::Error + Send + Sync>> {
        oas3::from_path(file)
            .map(|s| {
                s.paths
                    .iter()
                    .map(|(path_string, path)| {
                        create_tests(&s.servers, path_string, path, full, multistage)
                    })
                    .flatten()
                    .collect()
            })
            .map_err(|e| Box::from(e))
    }
}

fn create_tests_from_openapi_spec_imp(
    file: &str,
    full: bool,
    multistage: bool,
) -> Result<Vec<File>, Box<dyn Error + Send + Sync>> {
    openapi_v31::create_tests_from_openapi_spec(file, full, multistage).or(
        openapi_legacy::create_tests_from_openapi_spec(file, full, multistage),
    )
}

pub fn create_tests_from_openapi_spec(
    file: &str,
    full: bool,
    multistage: bool,
    output_path: Option<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match output_path {
        None => {
            error!("<NAME> of output is required for open api document ingestion.");
            Err(Box::new(GenericError {
                reason: "missing cli parameter".to_string(),
            }))
        }
        Some(p) => {
            let _ = std::fs::create_dir_all(&p)?;
            let root = std::path::PathBuf::from(&p);
            let tests = create_tests_from_openapi_spec_imp(file, full, multistage);
            let mut tests_generated = 0;
            let ret = tests.and_then(|f| {
                f.iter()
                    .map(|f| -> Result<(), Box<dyn Error + Send + Sync>> {
                        let file_path = root.join(f.filename.as_str());
                        std::fs::create_dir_all(file_path.parent().unwrap())?;
                        std::fs::File::create(file_path)
                            .map(|mut o| o.write(serde_yaml::to_string(f).unwrap().as_bytes()))
                            .map(|_| tests_generated += 1)
                            .map_err(|e| Box::from(e))
                    })
                    .collect::<Vec<Result<(), Box<dyn Error + Send + Sync>>>>()
                    .into_iter()
                    .collect()
            });
            match &ret {
                Ok(_) => info!("Tests generated:{tests_generated}"),
                Err(e) => error!("{e}"),
            }
            return ret;
        }
    }
}

pub async fn create_test_template(
    full: bool,
    multistage: bool,
    output: bool,
    name: Option<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let template = if full {
        serde_yaml::to_string(&template::template_full()?)?
    } else if multistage {
        serde_yaml::to_string(&template::template_staged()?)?
    } else {
        serde_yaml::to_string(&template::template()?)?
    };
    let template = template.replace("''", "");
    let mut result = String::default();

    for line in template.lines() {
        if !line.contains("null") {
            result = format!("{}{}\n", result, line)
        }
    }

    if output {
        info!("{}\n", result);
        Ok(())
    } else {
        match name {
            Some(n) => {
                let filename = if !n.ends_with(".jkt") {
                    format!("{}.jkt", n)
                } else {
                    n.clone()
                };

                if std::path::Path::new(&filename).exists() {
                    error!("`{}` already exists. Please pick a new name/location or delete the existing file.", filename);
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        "the output file already exists",
                    )));
                }

                let mut file = fs::File::create(&filename).await?;
                file.write_all(result.as_bytes()).await?;
                info!("Successfully created test (`{}`).\n", filename);
                Ok(())
            }
            None => {
                error!("<NAME> is required if not outputting to screen. `jk new <NAME>`");
                Err(Box::new(GenericError {
                    reason: "missing cli parameter".to_string(),
                }))
            }
        }
    }
}
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_status_code_number_ok() {
        assert_eq!(Some(204), create_status_code("204"));
    }

    #[test]
    fn create_status_code_pattern_ok() {
        assert_eq!(Some(200), create_status_code("2XX"));
    }

    #[test]
    fn create_status_code_non_number_none() {
        assert_eq!(None, create_status_code("Foo"));
    }

    #[test]
    fn filename_only_slash() {
        assert_eq!(
            format!("ROOT{}Get.jkt", std::path::MAIN_SEPARATOR_STR),
            create_filename("/", &test::http::Verb::Get)
        );
    }

    #[test]
    fn filename_one_component() {
        assert_eq!(
            format!("foo{}Delete.jkt", std::path::MAIN_SEPARATOR_STR),
            create_filename("/foo", &test::http::Verb::Delete)
        );
    }

    #[test]
    fn filename_multiple_components() {
        assert_eq!(
            format!(
                "foo{}bar{}Post.jkt",
                std::path::MAIN_SEPARATOR_STR,
                std::path::MAIN_SEPARATOR_STR
            ),
            create_filename("foo/bar/", &test::http::Verb::Post)
        );
    }

    #[test]
    fn filename_multiple_components_with_params() {
        assert_eq!(
            format!(
                "foo{0}bars{0}{{bar}}{0}Post.jkt",
                std::path::MAIN_SEPARATOR_STR
            ),
            create_filename("foo/bars/{bar}", &test::http::Verb::Post)
        );
    }
}
