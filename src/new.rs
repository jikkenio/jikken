use super::errors::GenericError;
use super::test::template;
use log::{error, info};

use std::error::Error;
use tokio::fs;
use tokio::io::AsyncWriteExt;

mod openapi_legacy {
    use crate::test;
    use crate::test::file::UnvalidatedRequest;
    use crate::test::file::UnvalidatedResponse;
    use crate::test::File;
    use openapiv3::v2::ReferenceOrSchema::Reference;
    use openapiv3::IndexMap;
    use openapiv3::{Operation, PathItem, RefOr, Responses, Server, VersionedOpenAPI};
    use std::collections::hash_map::RandomState;
    use std::io::BufReader;

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
        test_factory: impl Fn(&str, &openapiv3::Operation) -> Option<File>,
    ) -> Vec<File> {
        op.clone()
            .map(|op| {
                get_test_paths(root_servers, &path.servers, &op.servers, "{url}")
                    .iter()
                    .map(|url| test_factory(format!("{}{}", url, path_string).as_str(), &op))
                    .flatten()
                    .collect::<Vec<File>>()
            })
            .unwrap_or_default()
    }

    fn create_test(
        resolved_path: &str,
        op: &openapiv3::Operation,
        verb: test::http::Verb,
    ) -> Option<File> {
        let default = test::File::default();
        Some(File {
            name: op.summary.clone().or(default.name),
            id: op.operation_id.clone().or(default.id),
            tags: create_tags(&op.tags),
            request: Some(create_request(resolved_path, verb, op)),
            response: create_response(&op.responses).or(default.response),
            ..default
        })
    }

    fn create_get_test(resolved_path: &str, op: &openapiv3::Operation) -> Option<File> {
        create_test(resolved_path, op, test::http::Verb::Get)
    }

    fn create_post_test(resolved_path: &str, op: &openapiv3::Operation) -> Option<File> {
        create_test(resolved_path, op, test::http::Verb::Post)
    }

    fn create_delete_test(resolved_path: &str, op: &openapiv3::Operation) -> Option<File> {
        create_test(resolved_path, op, test::http::Verb::Delete)
    }

    fn create_put_test(resolved_path: &str, op: &openapiv3::Operation) -> Option<File> {
        create_test(resolved_path, op, test::http::Verb::Put)
    }

    fn create_patch_test(resolved_path: &str, op: &openapiv3::Operation) -> Option<File> {
        create_test(resolved_path, op, test::http::Verb::Patch)
    }

    fn create_tests(root_servers: &[Server], path_string: &str, path: &PathItem) -> Vec<File> {
        let stuff: [(&Option<Operation>, fn(&str, &Operation) -> Option<File>); 5] = [
            (&path.get, create_get_test),
            (&path.post, create_post_test),
            (&path.delete, create_delete_test),
            (&path.patch, create_patch_test),
            (&path.put, create_put_test),
        ];

        stuff
            .into_iter()
            .map(|(op, factory)| create_tests_for_op(op, root_servers, path, path_string, factory))
            .flatten()
            .collect()
    }

    pub fn create_tests_from_openapi_spec(file: &str) -> Option<Vec<File>> {
        let file = std::fs::File::open(file).ok()?;
        let reader = BufReader::new(file);

        let versioned_openapi: Result<VersionedOpenAPI, serde_json::Error> =
            serde_json::from_reader(reader);
        match versioned_openapi {
            Err(e) => {
                println!("ERROR IS {}", e);
                None
            }
            Ok(v) => {
                let openapi = v.upgrade();
                Some(
                    openapi
                        .paths
                        .iter()
                        .map(|(path_string, ref_or_path)| match ref_or_path {
                            RefOr::Item(path) => create_tests(&openapi.servers, path_string, path),
                            RefOr::Reference { .. } => Vec::default(),
                        })
                        .flatten()
                        .collect(),
                )
            }
        }
    }
}

mod openapi_v31 {
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

    fn create_test(
        resolved_path: &str,
        op: &oas3::spec::Operation,
        verb: test::http::Verb,
    ) -> Option<File> {
        let default = test::File::default();
        Some(File {
            name: op.summary.clone().or(default.name),
            id: op.operation_id.clone().or(default.id),
            tags: create_tags(&op.tags),
            request: Some(create_request(resolved_path, verb, op)),
            response: create_response(&op.responses).or(default.response),
            ..default
        })
    }

    fn create_get_test(resolved_path: &str, op: &oas3::spec::Operation) -> Option<File> {
        create_test(resolved_path, op, test::http::Verb::Get)
    }

    fn create_post_test(resolved_path: &str, op: &oas3::spec::Operation) -> Option<File> {
        create_test(resolved_path, op, test::http::Verb::Post)
    }

    fn create_delete_test(resolved_path: &str, op: &oas3::spec::Operation) -> Option<File> {
        create_test(resolved_path, op, test::http::Verb::Delete)
    }

    fn create_put_test(resolved_path: &str, op: &oas3::spec::Operation) -> Option<File> {
        create_test(resolved_path, op, test::http::Verb::Put)
    }

    fn create_patch_test(resolved_path: &str, op: &oas3::spec::Operation) -> Option<File> {
        create_test(resolved_path, op, test::http::Verb::Patch)
    }

    fn create_tests_for_op(
        op: &Option<Operation>,
        root_servers: &[Server],
        path: &PathItem,
        path_string: &str,
        test_factory: impl Fn(&str, &oas3::spec::Operation) -> Option<File>,
    ) -> Vec<File> {
        op.clone()
            .map(|op| {
                get_test_paths(root_servers, &path.servers, &op.servers, "{url}")
                    .iter()
                    .map(|url| test_factory(format!("{}{}", url, path_string).as_str(), &op))
                    .flatten()
                    .collect::<Vec<File>>()
            })
            .unwrap_or_default()
    }

    fn create_tests(root_servers: &[Server], path_string: &str, path: &PathItem) -> Vec<File> {
        let stuff: [(
            &Option<Operation>,
            fn(&str, &oas3::spec::Operation) -> Option<File>,
        ); 5] = [
            (&path.get, create_get_test),
            (&path.post, create_post_test),
            (&path.delete, create_delete_test),
            (&path.patch, create_patch_test),
            (&path.put, create_put_test),
        ];

        stuff
            .into_iter()
            .map(|(op, factory)| create_tests_for_op(op, root_servers, path, path_string, factory))
            .flatten()
            .collect()
    }

    pub fn create_tests_from_openapi_spec(file: &str) -> Option<Vec<File>> {
        let f = oas3::from_path(file);
        match f {
            Err(e) => {
                println!("ERROR IS {}", e);
                None
            }
            Ok(s) => Some(
                s.paths
                    .iter()
                    .map(|(path_string, path)| create_tests(&s.servers, path_string, path))
                    .flatten()
                    .collect(),
            ),
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
mod tests {
    use serde::Serialize;

    use super::*;

    fn get_spec_path(p: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test")
            .join("data_files")
            .join(p)
    }

    #[test]
    fn basic() {
        let tests = openapi_v31::create_tests_from_openapi_spec(
            get_spec_path("openapi1.json").to_str().unwrap(),
        );

        let ret = tests
            .map(|f| {
                f.iter()
                    .map(|f| format!("{}", serde_yaml::to_string(f).unwrap()))
                    .collect::<Vec<String>>()
                    .join("\n----------\n")
            })
            .unwrap_or_default();

        println!("\n\n\n{ret}");
    }

    #[test]
    fn basic2() {
        let tests = openapi_legacy::create_tests_from_openapi_spec(
            get_spec_path("bitbucket.json").to_str().unwrap(),
        );
        let ret = tests
            .map(|f| {
                f.iter()
                    .map(|f| format!("{}", serde_yaml::to_string(f).unwrap()))
                    .collect::<Vec<String>>()
                    .join("\n----------\n")
            })
            .unwrap_or_default();

        println!("\n\n\n{ret}");
    }
}
