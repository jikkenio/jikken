use super::errors::GenericError;
use super::test::template;
use log::{error, info};

use crate::test::file::NumericSpecification;
use crate::test::file::ValueOrNumericSpecification;
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

fn create_status_code(status_code_pattern: &str) -> Option<ValueOrNumericSpecification<u16>> {
    if status_code_pattern == "2XX" {
        Some(ValueOrNumericSpecification::Schema(NumericSpecification {
            specification: None,
            min: Some(200),
            max: Some(299),
        }))
    } else {
        status_code_pattern
            .parse()
            .ok()
            .map(ValueOrNumericSpecification::Value)
    }
}

fn create_filename(path_string: &str, verb: &http::Verb) -> String {
    let mut path = path_string
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join(std::path::MAIN_SEPARATOR_STR);

    if path.is_empty() {
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
    use crate::test::file;
    use crate::test::file::generate_value_from_schema;
    use crate::test::file::DatumSchema;
    use crate::test::file::FloatSpecification;
    use crate::test::file::IntegerSpecification;
    use crate::test::file::Specification;
    use crate::test::file::StringSpecification;
    use crate::test::file::UnvalidatedRequest;
    use crate::test::file::UnvalidatedResponse;
    use crate::test::file::UnvalidatedVariableNameOrDatumSchema;
    use crate::test::file::UnvalidatedVariableNameOrValue;
    use crate::test::file::ValueOrDatumSchema;
    use openapiv3::IndexMap;
    use openapiv3::OpenAPI;
    use openapiv3::Parameter;
    use openapiv3::Schema;
    use openapiv3::VariantOrUnknownOrEmpty;
    use openapiv3::{Operation, PathItem, RefOr, Responses, Server, VersionedOpenAPI};
    use std::collections::hash_map::RandomState;
    use std::collections::BTreeMap;
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

    fn create_response(responses: &Responses, spec: &OpenAPI) -> Option<UnvalidatedResponse> {
        responses
            .responses
            .iter()
            .map(|(sc, obj_or_ref)| (sc.to_string(), obj_or_ref))
            .filter(|(status_code_pattern, _)| status_code_pattern.starts_with('2'))
            .map(|(status_code_pattern, obj_or_ref)| {
                obj_or_ref.resolve(spec).ok().map(|t| {
                    let body_stuff = t.content.get("application/json").and_then(|content| {
                        content
                            .schema
                            .as_ref()
                            .and_then(|s| schema_to_datum(s.resolve(spec), spec))
                            .map(|ds| {
                                (
                                    generate_value_from_schema(&ds, 10)
                                        .map(UnvalidatedVariableNameOrValue::Component),
                                    UnvalidatedVariableNameOrDatumSchema::Component(ds),
                                )
                            })
                    });
                    UnvalidatedResponse {
                        status: create_status_code(status_code_pattern.as_str()),
                        headers: create_headers(&t.headers),
                        extract: None,
                        ignore: None,
                        strict: None,
                        body: body_stuff.clone().and_then(|(v, _)| v),
                        body_schema: None, //body_stuff.map(|(_, ds)| ds),
                    }
                })
            })
            .last()
            .flatten()
    }

    fn schema_to_datum(schema: &Schema, spec: &OpenAPI) -> Option<DatumSchema> {
        match &schema.kind {
            openapiv3::SchemaKind::Type(t) => match t {
                openapiv3::Type::Array(a) => {
                    let f = a
                        .items
                        .as_ref()
                        .and_then(|s| schema_to_datum(s.resolve(spec), spec));
                    Some(DatumSchema::List {
                        specification: Some(file::SequenceSpecification {
                            schema: f.map(|ds| {
                                file::ValuesOrSchema::Schemas(
                                    Specification::<Box<DatumSchema>>::Value(Box::from(ds)),
                                )
                            }),
                            min_length: a.min_items.map(|s| s as i64),
                            max_length: a.max_items.map(|s| s as i64),
                        }),
                    })
                }
                openapiv3::Type::Boolean {} => Some(DatumSchema::Boolean {
                    specification: None,
                }),
                openapiv3::Type::Integer(int) => Some(DatumSchema::Int {
                    specification: Some(IntegerSpecification {
                        max: int.maximum.map(|v| v + int.exclusive_maximum as i64),
                        min: int.minimum.map(|v| v + int.exclusive_minimum as i64),
                        ..Default::default()
                    }),
                }),
                openapiv3::Type::Number(num) => Some(DatumSchema::Float {
                    specification: Some(FloatSpecification {
                        max: num.maximum.map(|v| v + num.exclusive_maximum as i16 as f64),
                        min: num.minimum.map(|v| v + num.exclusive_minimum as i16 as f64),
                        ..Default::default()
                    }),
                }),
                openapiv3::Type::Object(obj) => {
                    let f = &obj
                        .properties
                        .iter()
                        .map(|(name, prop)| {
                            (name.clone(), schema_to_datum(prop.resolve(spec), spec))
                        })
                        .filter(|(_, s)| s.is_some())
                        .map(|(n, s)| (n, ValueOrDatumSchema::Datum(s.unwrap())))
                        .collect::<BTreeMap<String, ValueOrDatumSchema>>();

                    if f.is_empty() {
                        Some(DatumSchema::Object { schema: None })
                    } else {
                        Some(DatumSchema::Object {
                            schema: Some(f.clone()),
                        })
                    }
                }
                openapiv3::Type::String(string) => {
                    let string_spec = StringSpecification {
                        max_length: string.max_length.map(|s| s as i64),
                        min_length: string.min_length.map(|s| s as i64),
                        pattern: string.pattern.clone(),
                        ..Default::default()
                    };
                    match &string.format {
                        VariantOrUnknownOrEmpty::<openapiv3::StringFormat>::Item(s) => match s {
                            openapiv3::StringFormat::Date => Some(DatumSchema::Date {
                                specification: None,
                            }),
                            openapiv3::StringFormat::DateTime => Some(DatumSchema::DateTime {
                                specification: None,
                            }),
                            _ => Some(DatumSchema::String {
                                specification: Some(string_spec),
                            }),
                        },
                        _ => Some(DatumSchema::String {
                            specification: Some(string_spec),
                        }),
                    }
                }
            },
            _ => None,
        }
    }

    fn create_request(
        url: &str,
        verb: test::http::Verb,
        op: &openapiv3::Operation,
        spec: &OpenAPI,
    ) -> UnvalidatedRequest {
        let mut headers: Vec<test::http::Header> = vec![];
        let mut parameters: Vec<test::http::Parameter> = vec![];

        op.parameters.iter().for_each(|f| {
            if let Ok(Parameter { data, kind }) = f.resolve(spec) {
                match &kind {
                    openapiv3::ParameterKind::Query { .. } => {
                        parameters.push(test::http::Parameter {
                            param: data.name.clone(),
                            value: String::default(),
                            matches_variable: std::cell::Cell::new(false),
                        })
                    }
                    openapiv3::ParameterKind::Header { .. } => headers.push(test::http::Header {
                        header: data.name.clone(),
                        value: String::default(),
                        matches_variable: std::cell::Cell::new(false),
                    }),
                    openapiv3::ParameterKind::Path { .. } => (), //user will have to do this themselves, based upon generated template
                    openapiv3::ParameterKind::Cookie { .. } => (), //no cookie support
                }
            }
        });

        let body_schema = op.request_body.as_ref().and_then(|maybe_request| {
            maybe_request.resolve(spec).ok().and_then(|r| {
                r.content.get("application/json").and_then(|content| {
                    content
                        .schema
                        .as_ref()
                        .and_then(|s| schema_to_datum(s.resolve(spec), spec))
                        .map(|ds| {
                            (
                                generate_value_from_schema(&ds, 10)
                                    .map(UnvalidatedVariableNameOrValue::Component),
                                UnvalidatedVariableNameOrDatumSchema::Component(ds),
                            )
                        })
                })
            })
        });

        UnvalidatedRequest {
            body: body_schema.clone().and_then(|(v, _)| v),
            body_schema: None, //body_schema.map(|(_, ds)| ds),
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
                Some(servers.iter().map(|s| s.url.clone()).collect())
            }
        };

        url_extractor(path_servers)
            .or(url_extractor(root_servers))
            .or(url_extractor(op_servers))
            .unwrap_or(vec![fallback.to_string()])
    }

    fn create_tests_for_op(
        op: &Option<Operation>,
        path: &PathItem,
        path_string: &str,
        verb: test::http::Verb,
        full: bool,
        multistage: bool,
        spec: &OpenAPI,
    ) -> Vec<File> {
        op.clone()
            .map(|op| {
                get_test_paths(&spec.servers, &path.servers, &op.servers, "{url}")
                    .iter()
                    .flat_map(|url| {
                        create_test(
                            format!("{}{}", url, path_string).as_str(),
                            &op,
                            verb,
                            full,
                            multistage,
                            path_string,
                            spec,
                        )
                    })
                    .collect::<Vec<File>>()
            })
            .unwrap_or_default()
    }

    fn create_variables(
        op: &openapiv3::Operation,
        spec: &OpenAPI,
    ) -> Option<Vec<test::file::UnvalidatedVariable>> {
        let ret = op
            .parameters
            .iter()
            .filter_map(|p_or_ref| {
                p_or_ref
                    .resolve(spec)
                    .map(|t| test::file::UnvalidatedVariable {
                        name: t.name.clone(),
                        value: test::file::ValueOrDatumOrFile::Value {
                            value: serde_json::Value::from("value".to_string()),
                        },
                    })
                    .ok()
            })
            .collect::<Vec<file::UnvalidatedVariable>>();

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
        spec: &OpenAPI,
    ) -> Option<File> {
        let default = if full {
            test::template::template_full().unwrap()
        } else if multistage {
            test::template::template_staged().unwrap()
        } else {
            test::File::default()
        };

        let resolved_path = path.replace('{', "${").to_string();
        let request = create_request(resolved_path.as_str(), verb, op, spec);
        let response =
            create_response(&op.responses, spec).or(Some(UnvalidatedResponse::default()));
        let variables = create_variables(op, spec);

        if multistage || verb == test::http::Verb::Delete {
            Some(File {
                name: op.summary.clone().or(default.name),
                description: op.description.clone(),
                id: op.operation_id.clone().or(default.id),
                tags: create_tags(&op.tags),
                stages: Some(vec![test::file::UnvalidatedStage {
                    request,
                    compare: None,
                    response,
                    variables,
                    name: None,
                    delay: None,
                }]),
                filename: create_filename(path_string, &verb),
                ..default
            })
        } else {
            Some(File {
                name: op.summary.clone().or(default.name),
                description: op.description.clone(),
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
        path_string: &str,
        path: &PathItem,
        full: bool,
        multistage: bool,
        spec: &OpenAPI,
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
            .flat_map(|(op, verb)| {
                create_tests_for_op(op, path, path_string, verb, full, multistage, spec)
            })
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
                    .flat_map(|(path_string, ref_or_path)| match ref_or_path {
                        RefOr::Item(path) => {
                            create_tests(path_string, path, full, multistage, &openapi)
                        }
                        RefOr::Reference { .. } => Vec::default(),
                    })
                    .collect())
            }
        }
    }
}

mod openapi_v31 {
    use super::*;
    use crate::test;
    use crate::test::file::DateSpecification;
    use crate::test::file::DateTimeSpecification;
    use crate::test::file::DatumSchema;
    use crate::test::file::EmailSpecification;
    use crate::test::file::FloatSpecification;
    use crate::test::file::IntegerSpecification;
    use crate::test::file::Specification;
    use crate::test::file::StringSpecification;
    use crate::test::file::UnvalidatedRequest;
    use crate::test::file::UnvalidatedResponse;
    use crate::test::file::UnvalidatedVariableNameOrDatumSchema;
    use crate::test::file::ValueOrDatumSchema;
    use oas3::spec::Header;
    use oas3::spec::ObjectOrReference;
    use oas3::spec::Operation;
    use oas3::spec::PathItem;
    use oas3::spec::Response;
    use oas3::spec::Server;
    use oas3::spec::Spec;
    use std::collections::BTreeMap;
    use test::file::SequenceSpecification;
    use test::file::ValuesOrSchema;

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
                Some(servers.iter().map(|s| s.url.clone()).collect())
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
        spec: &Spec,
    ) -> Option<UnvalidatedResponse> {
        responses
            .iter()
            .filter(|(status_code_pattern, _)| status_code_pattern.starts_with('2'))
            .map(|(status_code_pattern, obj_or_ref)| {
                obj_or_ref.resolve(spec).ok().map(|t| UnvalidatedResponse {
                    status: create_status_code(status_code_pattern),
                    body: None,
                    headers: create_headers(&t.headers),
                    extract: None,
                    ignore: None,
                    strict: None,
                    body_schema: t.content.get("application/json").and_then(|c| {
                        c.schema(spec).ok().and_then(|s| {
                            schema_to_datum(s, spec)
                                .map(UnvalidatedVariableNameOrDatumSchema::Component)
                        })
                    }),
                })
            })
            .last()
            .flatten()
    }

    fn schema_to_datum(schema: oas3::Schema, spec: &Spec) -> Option<DatumSchema> {
        schema.schema_type.map(|t| match t {
            oas3::spec::SchemaType::Array => DatumSchema::List {
                specification: Some(SequenceSpecification {
                    schema: schema.items.and_then(|items| {
                        items.resolve(spec).ok().and_then(|s| {
                            schema_to_datum(s, spec).map(|ds| {
                                ValuesOrSchema::Schemas(Specification::<Box<DatumSchema>>::Value(
                                    Box::from(ds),
                                ))
                            })
                        })
                    }),
                    max_length: schema.max_items.map(|n| n as i64),
                    min_length: schema.min_items.map(|n| n as i64),
                }),
            },
            oas3::spec::SchemaType::Boolean => DatumSchema::Boolean {
                specification: None,
            },
            oas3::spec::SchemaType::Integer => DatumSchema::Int {
                specification: Some(IntegerSpecification {
                    max: schema.maximum.and_then(|n| {
                        n.as_i64()
                            .map(|n| n + schema.exclusive_maximum.unwrap_or_default() as i64)
                    }),
                    min: schema.minimum.and_then(|n| {
                        n.as_i64()
                            .map(|n| n + schema.exclusive_minimum.unwrap_or_default() as i64)
                    }),
                    ..Default::default()
                }),
            },
            oas3::spec::SchemaType::Number => DatumSchema::Float {
                specification: Some(FloatSpecification {
                    max: schema.maximum.and_then(|n| {
                        n.as_f64()
                            .map(|n| n + schema.exclusive_maximum.unwrap_or_default() as i64 as f64)
                    }),
                    min: schema.minimum.and_then(|n| {
                        n.as_f64()
                            .map(|n| n + schema.exclusive_minimum.unwrap_or_default() as i64 as f64)
                    }),
                    ..Default::default()
                }),
            },
            oas3::spec::SchemaType::Object => DatumSchema::Object {
                schema: Some(
                    schema
                        .properties
                        .iter()
                        .filter_map(|(k, maybe_schema)| {
                            maybe_schema
                                .resolve(spec)
                                .ok()
                                .map(|v| {
                                    (
                                        k.clone(),
                                        schema_to_datum(v, spec).map(ValueOrDatumSchema::Datum),
                                    )
                                })
                                .filter(|(_, ds)| ds.is_some())
                                .map(|(n, ds)| (n, ds.unwrap()))
                        })
                        .collect::<BTreeMap<String, ValueOrDatumSchema>>(),
                ),
            },
            oas3::spec::SchemaType::String => {
                let string_spec = StringSpecification {
                    pattern: schema.pattern,
                    max_length: schema.max_length.map(|n| n as i64),
                    min_length: schema.min_length.map(|n| n as i64),
                    ..Default::default()
                };

                match schema.format.unwrap_or_default().as_str() {
                    "date" => DatumSchema::Date {
                        specification: Some(DateSpecification {
                            ..Default::default()
                        }),
                    },
                    "date-time" => DatumSchema::DateTime {
                        specification: Some(DateTimeSpecification {
                            ..Default::default()
                        }),
                    },
                    "email" => DatumSchema::Email {
                        specification: Some(EmailSpecification {
                            specification: string_spec,
                        }),
                    },
                    _ => DatumSchema::String {
                        specification: Some(string_spec),
                    },
                }
            }
        })
    }

    fn create_request(
        url: &str,
        verb: test::http::Verb,
        op: &oas3::spec::Operation,
        spec: &Spec,
    ) -> UnvalidatedRequest {
        let mut headers: Vec<test::http::Header> = vec![];
        let mut parameters: Vec<test::http::Parameter> = vec![];

        op.parameters.iter().for_each(|f| {
            if let Ok(t) = f.resolve(spec) {
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
                    "cookie" => (), //These will get picked up automatically as state vars
                    _ => (),
                }
            }
        });

        let maybe_schema = op.request_body.as_ref().and_then(|body| {
            body.resolve(spec).ok().and_then(|b| {
                b.content.get("application/json").and_then(|c| {
                    c.schema(spec).ok().and_then(|s| {
                        schema_to_datum(s, spec)
                            .map(UnvalidatedVariableNameOrDatumSchema::Component)
                    })
                })
            })
        });

        UnvalidatedRequest {
            body: None,
            body_schema: maybe_schema,
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

    fn create_variables(
        op: &Operation,
        spec: &Spec,
    ) -> Option<Vec<test::file::UnvalidatedVariable>> {
        let ret = op
            .parameters
            .iter()
            .map(|p_or_ref| {
                p_or_ref
                    .resolve(spec)
                    .ok()
                    .map(|t| test::file::UnvalidatedVariable {
                        name: t.name.clone(),
                        value: test::file::ValueOrDatumOrFile::Value {
                            value: serde_json::Value::from("".to_string()),
                        },
                    })
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
        spec: &Spec,
    ) -> Option<File> {
        let default = if full {
            test::template::template_full().unwrap()
        } else if multistage {
            test::template::template_staged().unwrap()
        } else {
            test::File::default()
        };

        //openapi spec describes path paramters as /foo/{myVar}
        //change that to fit how Jikken specifies variables
        //and create jikken variables for each
        let resolved_path = path.replace('{', "${").to_string();
        let variables = create_variables(op, spec);

        let request = create_request(resolved_path.as_str(), verb, op, spec);
        let response =
            create_response(&op.responses, spec).or(Some(UnvalidatedResponse::default()));

        if multistage || verb == test::http::Verb::Delete {
            Some(File {
                name: op.summary.clone().or(default.name),
                description: op.description.clone(),
                id: op.operation_id.clone().or(default.id),
                tags: create_tags(&op.tags),
                stages: Some(vec![test::file::UnvalidatedStage {
                    request,
                    compare: None,
                    response,
                    variables,
                    name: None,
                    delay: None,
                }]),
                filename: create_filename(path_string, &verb),
                ..default
            })
        } else {
            Some(File {
                description: op.description.clone(),
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
        path: &PathItem,
        path_string: &str,
        verb: test::http::Verb,
        full: bool,
        multistage: bool,
        spec: &Spec,
    ) -> Vec<File> {
        op.clone()
            .map(|op| {
                get_test_paths(&spec.servers, &path.servers, &op.servers, "${url}")
                    .into_iter()
                    .filter_map(|url| {
                        create_test(
                            format!("{}{}", url, path_string).as_str(),
                            &op,
                            verb,
                            full,
                            multistage,
                            path_string,
                            spec,
                        )
                    })
                    .collect::<Vec<File>>()
            })
            .unwrap_or_default()
    }

    fn create_tests(
        path_string: &str,
        path: &PathItem,
        full: bool,
        multistage: bool,
        spec: &Spec,
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
            .flat_map(|(op, verb)| {
                create_tests_for_op(op, path, path_string, verb, full, multistage, spec)
            })
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
                    .flat_map(|(path_string, path)| {
                        create_tests(path_string, path, full, multistage, &s)
                    })
                    .collect()
            })
            .map_err(Box::from)
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
            std::fs::create_dir_all(&p)?;
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
                            .map_err(Box::from)
                    })
                    .collect::<Vec<Result<(), Box<dyn Error + Send + Sync>>>>()
                    .into_iter()
                    .collect()
            });
            match &ret {
                Ok(_) => info!("Tests generated:{tests_generated}"),
                Err(e) => error!("{e}"),
            }

            ret
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
    //use crate::test::file::Specification;
    use crate::test::file::ValueOrNumericSpecification;

    #[test]
    fn create_status_code_number_ok() {
        assert_eq!(
            Some(ValueOrNumericSpecification::Value(204)),
            create_status_code("204")
        );
    }

    #[test]
    fn create_status_code_pattern_ok() {
        assert_eq!(
            Some(ValueOrNumericSpecification::Schema(NumericSpecification {
                specification: None,
                min: Some(200),
                max: Some(299),
            })),
            create_status_code("2XX")
        );
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
