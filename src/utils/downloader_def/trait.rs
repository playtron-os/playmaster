use std::path::Path;

use crate::utils::errors::ResultWithError;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ArtifactInfo {
    pub name: String,
    pub size: Option<u64>,
    pub download_url: Option<String>,
}

pub trait SourceProvider {
    fn list_artifacts(&self) -> ResultWithError<Vec<ArtifactInfo>>;
    fn download_artifact(&self, artifact_name: &str, dest_path: &Path) -> ResultWithError<()>;
}
