use hyper::Request;
use bytes::{Bytes, BytesMut};
use http_body_util::{BodyExt, Empty};
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use log::{debug, error, info, warn};
use regex::Regex;
use remove_dir_all::remove_dir_all;
use serde::Deserialize;
use std::cmp::Ordering;
use std::env;
use std::error::Error;
use std::io::{stdout, Cursor, Write};
use tokio::io::AsyncWriteExt;

const UPDATE_URL: &str = "https://api.jikken.io/v1/latest_version";
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
pub struct ReleaseResponse {
    pub version: String,
    pub url: String,
}

#[derive(Eq)]
pub struct Version(String);

impl Version {
    //Extraneous trailing zeros can throw a wrench in things
    fn normalized(&self) -> String {
        let trailing_zero_regex: Regex = Regex::new(r"(.0)?$").unwrap();

        let ret = trailing_zero_regex
            .find(self.0.as_str())
            .map(|mat| self.0[..mat.range().start].to_string())
            .unwrap_or(self.0.clone());

        ret
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        let lhs_val = self.normalized();
        let rhs_val = other.normalized();

        lhs_val == rhs_val
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let lhs_val = self.normalized();
        let rhs_val = other.normalized();

        if lhs_val > rhs_val {
            return Ordering::Greater;
        }

        if lhs_val < rhs_val {
            return Ordering::Less;
        }

        Ordering::Equal
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

async fn update(url: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("Jikken is updating to the latest version...");
    stdout().flush().unwrap();

    let file_name_opt = url.split('/').last();

    if file_name_opt.is_none() {
        error!("error: invalid url");
        return Ok(());
    }

    let tmp_dir = tempfile::Builder::new().tempdir_in(::std::env::current_dir()?)?;

    let tmp_tarball_path = tmp_dir.path().join(file_name_opt.unwrap());

    let mut tmp_tarball = tokio::fs::File::create(&tmp_tarball_path).await?;
    let response = reqwest::get(url).await?;
    let mut content = Cursor::new(response.bytes().await?);
    let save_file_reuslts = tmp_tarball.write_all_buf(&mut content).await;

    if let Err(error) = save_file_reuslts {
        error!("error saving downloaded file: {}", error);
        return Ok(());
    }

    if env::consts::OS == "windows" {
        self_update::Extract::from_source(&tmp_tarball_path)
            .archive(self_update::ArchiveKind::Zip)
            .extract_into(tmp_dir.path())?;
    } else {
        self_update::Extract::from_source(&tmp_tarball_path)
            .archive(self_update::ArchiveKind::Tar(Some(
                self_update::Compression::Gz,
            )))
            .extract_into(tmp_dir.path())?;
    }

    let tmp_file = tmp_dir.path().join("replacement_tmp");
    let bin_path = match env::consts::OS {
        "windows" => tmp_dir.path().join("jk.exe"),
        _ => tmp_dir.path().join("jk"),
    };
    self_update::Move::from_source(&bin_path)
        .replace_using_temp(&tmp_file)
        .to_dest(&::std::env::current_exe()?)?;

    drop(tmp_tarball);
    _ = remove_dir_all(tmp_dir);

    Ok(())
}

fn has_newer_version(new_version: Version) -> bool {
    new_version > Version(crate::VERSION.to_string())
}

pub async fn get_latest_version() -> Result<Option<ReleaseResponse>, Box<dyn Error + Send + Sync>> {
    let client: Client<_, Empty<Bytes>> = Client::builder(TokioExecutor::new()).build(crate::telemetry::get_connector());
    let req = Request::builder()
        .uri(format!(
            "{}?channel=stable&platform={}",
            UPDATE_URL,
            env::consts::OS
        ))
        .body(Empty::new())?;

    let resp = client.request(req).await?;
    let (_, mut body) = resp.into_parts();

    let mut body_bytes = BytesMut::new();

        while let Some(next) = body.frame().await {
            let frame = next.unwrap();
            if let Some(chunk) = frame.data_ref() {
                body_bytes.extend(chunk);
            }
        }
    if let Ok(r) = serde_json::from_slice::<ReleaseResponse>(&body_bytes) {
        if has_newer_version(Version(r.version.clone())) {
            return Ok(Some(r));
        }
    }

    Ok(None)
}

pub async fn try_updating() {
    let latest_version = get_latest_version().await;

    match latest_version {
        Ok(lv_opt) => {
            if let Some(lv) = lv_opt {
                match update(&lv.url).await {
                    Ok(_) => {
                        info!("update completed\n");
                    }
                    Err(error) => {
                        error!(
                            "Jikken encountered an error when trying to update itself: {}",
                            error
                        );
                    }
                }
                return;
            }
        }
        Err(error) => {
            debug!("error checking for updates: {}", error);
        }
    }

    error!("Jikken was unable to find an update for this platform and release channel");
}

pub async fn check_for_updates() {
    match get_latest_version().await {
        Ok(latest_version) => {
            if let Some(latest) = latest_version {
                warn!(
                    "Jikken found new version ({}), currently running version ({})",
                    latest.version, VERSION
                );
                warn!("Run command: `jk update` to update jikken or update using your package manager");
            }
        }
        Err(error) => {
            debug!("error checking for updates: {}", error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_less() -> () {
        assert!(Version("0.6.1".to_string()) < Version("0.7.0".to_string()));
    }

    #[test]
    fn version_greater() -> () {
        assert!(Version("0.7.1".to_string()) > Version("0.7.0".to_string()));
    }

    #[test]
    fn version_greater_or_equal() -> () {
        assert!(Version("0.7.1".to_string()) >= Version("0.7.0".to_string()));
    }

    #[test]
    fn version_less_or_equal() -> () {
        assert!(Version("0.6.0.0".to_string()) <= Version("0.7.0".to_string()));
    }

    #[test]
    fn version_equal() -> () {
        assert!(Version("0.6.1".to_string()) == Version("0.6.1".to_string()));
        assert!(Version("0.6.1".to_string()) == Version("0.6.1.0".to_string()));
    }

    #[test]
    fn version_not_equal() -> () {
        assert!(Version("0.6.1".to_string()) != Version("0.6.2".to_string()));
    }

    #[test]
    fn newer_version_checkings() -> () {
        assert!(!has_newer_version(Version("0.6.1".to_string())));
    }
} //mod tests
