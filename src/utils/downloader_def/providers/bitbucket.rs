use reqwest::blocking::{Client, Response};
use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde::Deserialize;
use std::fs::File;
use std::io::copy;
use std::path::Path;

use crate::utils::downloader_def::r#trait::{ArtifactInfo, SourceProvider};
use crate::utils::errors::ResultWithError;

#[derive(Debug)]
pub struct BitbucketSourceProvider {
    client: Client,
    workspace: String,
    repo: String,
    token: Option<String>,
}

impl BitbucketSourceProvider {
    pub fn new<S: Into<String>>(workspace: S, repo: S, token: Option<String>) -> Self {
        let client = Client::builder()
            .build()
            .expect("Failed to create HTTP client");
        Self {
            client,
            workspace: workspace.into(),
            repo: repo.into(),
            token,
        }
    }

    fn apply_auth(
        &self,
        req: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        if let Some(tok) = &self.token {
            req.header(AUTHORIZATION, format!("Bearer {}", tok))
        } else {
            req
        }
    }

    fn save_to_file(&self, mut resp: Response, dest: &Path) -> ResultWithError<()> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = File::create(dest)?;
        copy(&mut resp, &mut file)?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct BitbucketDownloadEntry {
    name: String,
    size: Option<u64>,
    links: BitbucketLinks,
}

#[derive(Debug, Deserialize)]
struct BitbucketLinks {
    #[serde(rename = "self")]
    self_link: BitbucketLink,
}

#[derive(Debug, Deserialize)]
struct BitbucketLink {
    href: String,
}

impl SourceProvider for BitbucketSourceProvider {
    fn list_artifacts(&self) -> ResultWithError<Vec<ArtifactInfo>> {
        let url = format!(
            "https://api.bitbucket.org/2.0/repositories/{}/{}/downloads",
            self.workspace, self.repo
        );

        let mut req = self.client.get(&url).header(ACCEPT, "application/json");
        req = self.apply_auth(req);
        let resp = req.send()?;
        if !resp.status().is_success() {
            return Err(format!("Bitbucket returned status {}", resp.status()).into());
        }

        #[derive(Deserialize)]
        struct ListResponse {
            values: Vec<BitbucketDownloadEntry>,
        }
        let lr: ListResponse = resp.json()?;
        Ok(lr
            .values
            .into_iter()
            .map(|v| ArtifactInfo {
                name: v.name,
                size: v.size,
                download_url: Some(v.links.self_link.href),
            })
            .collect())
    }

    fn download_artifact(&self, artifact_name: &str, dest_path: &Path) -> ResultWithError<()> {
        let url = format!(
            "https://api.bitbucket.org/2.0/repositories/{}/{}/downloads/{}",
            self.workspace, self.repo, artifact_name
        );

        let mut req = self.client.get(&url);
        req = self.apply_auth(req);
        let resp = req.send()?;
        if !resp.status().is_success() {
            return Err(format!("Failed to download {}: {}", artifact_name, resp.status()).into());
        }

        self.save_to_file(resp, dest_path)?;
        Ok(())
    }
}
