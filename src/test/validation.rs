use crate::test;
use crate::test::definition;
use crate::test::variable;
use std::fmt;
use std::path::PathBuf;

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
    _file: &test::File,
    _global_variables: &[test::Variable],
) -> Result<bool, Error> {
    Ok(true)
}

// this method is intended to do basic validation of the test file and convert it into a TestDefinition if it passes
pub fn validate_file(
    file: test::File,
    global_variables: &[test::Variable],
    project: Option<String>,
    environment: Option<String>,
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

    let generated_id = file.generate_id();

    let variables = test::Variable::validate_variables_opt(
        file.variables,
        PathBuf::from(&file.filename)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or(&file.filename),
    )?;

    let variables2 = test::Variable::validate_variables_opt2(
        file.variables2,
        PathBuf::from(&file.filename)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or(&file.filename),
    )?;

    let td = test::Definition {
        name: file.name,
        description: file.description,
        id: file.id.unwrap_or(generated_id).to_lowercase(),
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
        filename: file.filename,
    };

    td.update_variable_matching();
    Ok(td)
}

// this method is intended to do a thorough validation of rules and logic in the resolved test definition
pub fn _validate_definition(_test_definition: &test::Definition) -> Result<bool, Error> {
    Ok(true)
}
