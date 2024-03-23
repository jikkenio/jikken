use crate::test;
use crate::test::file::UnvalidatedRequest;
use crate::test::file::UnvalidatedResponse;
use crate::test::Definition;
use crate::test::File;

use super::errors::GenericError;
use super::test::template;
use log::{error, info};
use oas3;
use oas3::spec::Header;
use oas3::spec::ObjectOrReference;
use oas3::spec::Operation;
use oas3::spec::PathItem;
use oas3::spec::Response;
use oas3::spec::Server;
use std::collections::BTreeMap;
use std::error::Error;
use std::str::FromStr;
use tokio::fs;
use tokio::io::AsyncWriteExt;

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
            value: "".to_string(),
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
        .filter(|(status_code_pattern, obj_or_ref)| status_code_pattern.starts_with("2"))
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
    //If you have url parameters in your path, you have to do this a bit diffferently
    //create_test may need to change to return multiple? we resovle the path prior!
    let mut parameters: Vec<test::http::Parameter> = vec![];

    op.parameters.iter().for_each(|f| match f {
        ObjectOrReference::Object(t) => {
            match t.location.as_str() {
                "query" => parameters.push(test::http::Parameter {
                    param: t.name.clone(),
                    value: "".to_string(),
                    matches_variable: std::cell::Cell::new(false),
                }),
                "header" => headers.push(test::http::Header {
                    header: t.name.clone(),
                    value: "".to_string(),
                    matches_variable: std::cell::Cell::new(false),
                }),
                "path" => (),   //we have to handle this higher up
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
    println!("CREATING TEST {resolved_path}");

    Some(File {
        cleanup: None,
        compare: None,
        disabled: None,
        env: None,
        name: None,
        id: None,
        project: None,
        tags: create_tags(&op.tags),
        requires: None,
        filename: "".to_string(),
        iterate: None,
        variables: None,
        stages: None,
        setup: None,
        request: Some(create_request(resolved_path, verb, op)),
        response: create_response(&op.responses),
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
            get_test_paths(root_servers, &path.servers, &op.servers, "$url")
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
                .map(|(path_string, path)| {
                    println!("IN PATH: {path_string}");
                    create_tests(&s.servers, path_string, path)
                })
                .flatten()
                .collect(),
        ),
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
    let mut result = "".to_string();

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
        println!("HERE");
        //.to_str().unwrap();
        let tests =
            create_tests_from_openapi_spec(get_spec_path("openapi1.json").to_str().unwrap());

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
