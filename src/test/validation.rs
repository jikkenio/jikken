use crate::test;
use crate::test::definition;
use crate::test::validation;
use crate::test::variable;
use log::warn;
use regex::Regex;
use std::fmt;
use std::path::PathBuf;
use ulid::Ulid;

#[derive(Debug, Clone)]
pub struct Error {
    pub reason: String,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.reason.is_empty() {
            write!(f, "test validation")
        } else {
            write!(f, "{}", self.reason)
        }
    }
}

fn validate_test_file(
    file: &test::File,
    _global_variables: &[test::Variable],
) -> Result<bool, Error> {
    if !file
        .platform_id
        .clone()
        .map(|ulid| Ulid::from_string(&ulid).is_ok())
        .unwrap_or(true)
    {
        warn!("Test file ({}) has invalid platform identifier ({}). PlatformId must be empty or a valid ULID.", file.filename, file.platform_id.clone().unwrap_or("".to_string()));
    }

    let regex = Regex::new(r"(?i)^[a-z0-9-_]+$").unwrap();
    if !file
        .id
        .clone()
        .map(|id| regex.is_match(id.as_str()))
        .unwrap_or(true)
    {
        return Err(validation::Error {
            reason:
                format!("id '{}' is invalid - may only contain alphanumeric characters, hyphens, and underscores", file.id.clone().unwrap())
                    .to_string(),
        });
    }

    Ok(true)
}

// this method is intended to do basic validation of the test file and convert it into a TestDefinition if it passes
pub fn validate_file(
    file: test::File,
    global_variables: &[test::Variable],
    project: Option<String>,
    environment: Option<String>,
    index: usize,
) -> Result<test::Definition, Error> {
    validate_test_file(&file, global_variables)?;
    let new_tags = if let Some(tags) = file.tags.as_ref() {
        tags.to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    };

    let variables = test::Variable::validate_variables_opt(
        file.clone().variables,
        PathBuf::from(&file.filename)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or(&file.filename),
    )?;

    let variables2 = test::Variable::validate_variables_opt2(
        file.clone().variables2,
        PathBuf::from(&file.filename)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or(&file.filename),
    )?;

    let td = test::Definition {
        file_data: file.clone(),
        name: file.name,
        description: file.description,
        id: file.id.map(|i| i.to_lowercase()),
        platform_id: file.platform_id,
        project: file.project.or(project),
        environment: file.env.or(environment),
        requires: file.requires,
        tags: new_tags,
        iterate: file.iterate.unwrap_or(1),
        variables: variables.clone(),
        variables2,
        global_variables: global_variables.to_vec(),
        stages: definition::StageDescriptor::validate_stages_opt(
            file.request,
            file.compare,
            file.response,
            file.stages,
            &variable::parse_source_path(&file.filename),
            &variables,
        )?,
        setup: definition::RequestResponseDescriptor::new_opt(file.setup, &variables)?,
        cleanup: definition::CleanupDescriptor::new(file.cleanup, &variables)?,
        disabled: file.disabled.unwrap_or_default(),
        index,
    };

    td.update_variable_matching();
    Ok(td)
}

// this method is intended to do a thorough validation of rules and logic in the resolved test definition
pub fn _validate_definition(_test_definition: &test::Definition) -> Result<bool, Error> {
    Ok(true)
}
