use hyper::{body, Body, Client, Request};
use hyper_tls::HttpsConnector;
use log::{error, info};
use remove_dir_all::remove_dir_all;
use serde::Deserialize;
use std::env;
use std::error::Error;
use std::io::{stdout, Cursor, Write};
use tokio::io::AsyncWriteExt;

const UPDATE_URL: &str = "https://api.jikken.io/v1/latest_version";

#[derive(Deserialize)]
pub struct ReleaseResponse {
    pub version: String,
    pub url: String,
}

pub async fn update(url: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("Jikken is updating to the latest version...");
    stdout().flush().unwrap();

    let file_name_opt = url.split("/").last();

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
            .extract_into(&tmp_dir.path())?;
    } else {
        self_update::Extract::from_source(&tmp_tarball_path)
            .archive(self_update::ArchiveKind::Tar(Some(
                self_update::Compression::Gz,
            )))
            .extract_into(&tmp_dir.path())?;
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

fn has_newer_version(new_version: String) -> bool {
    let new_version_segments: Vec<&str> = new_version.split(".").collect();
    let my_version_segments: Vec<&str> = crate::VERSION.split(".").collect();

    let segment_length = std::cmp::min(new_version_segments.len(), my_version_segments.len());

    for i in 0..segment_length {
        let new_segment_opt = new_version_segments[i].parse::<u32>();
        let my_segment_opt = my_version_segments[i].parse::<u32>();

        if new_segment_opt.is_err() || my_segment_opt.is_err() {
            return false;
        } else {
            if new_segment_opt.unwrap() > my_segment_opt.unwrap() {
                return true;
            }
        }
    }

    false
}

pub async fn check_for_updates() -> Result<Option<ReleaseResponse>, Box<dyn Error + Send + Sync>> {
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    let req = Request::builder()
        .uri(format!(
            "{}?channel=stable&platform={}",
            UPDATE_URL,
            env::consts::OS
        ))
        .body(Body::empty())?;

    let resp = client.request(req).await?;
    let (_, body) = resp.into_parts();
    let response_bytes = body::to_bytes(body).await?;
    if let Ok(r) = serde_json::from_slice::<ReleaseResponse>(&response_bytes.to_vec()) {
        if has_newer_version(r.version.clone()) {
            return Ok(Some(r));
        }
    }

    Ok(None)
}
