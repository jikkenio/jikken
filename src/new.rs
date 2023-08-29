use super::errors::GenericError;
use super::test::template;
use log::{error, info};
use std::error::Error;
use tokio::fs;
use tokio::io::AsyncWriteExt;

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
