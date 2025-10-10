use std::path::Path;

use crate::utils::{
    downloader_def::r#trait::{ArtifactInfo, SourceProvider},
    errors::{EmptyResult, ResultWithError},
    semver::SemverUtils,
};

pub struct Downloader<P: SourceProvider> {
    provider: P,
}

impl<P: SourceProvider> Downloader<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    pub fn list(&self) -> ResultWithError<Vec<ArtifactInfo>> {
        self.provider.list_artifacts()
    }

    pub fn download(&self, artifact_name: &str, dest_path: &Path) -> EmptyResult {
        self.provider.download_artifact(artifact_name, dest_path)
    }

    pub fn get_versioned_artifact(
        &self,
        version: Option<String>,
    ) -> ResultWithError<Option<ArtifactInfo>> {
        let artifacts = self.list()?;
        let aarch = crate::utils::os::OsUtils::detect_arch();
        let version = version.and_then(|version| SemverUtils::extract_version(&version));

        // Filter for RPM artifacts for the detected architecture
        let mut matching: Vec<_> = artifacts
            .into_iter()
            .filter(|a| a.name.ends_with(&format!("{}.rpm", aarch)))
            .collect();

        if matching.is_empty() {
            return Err(format!("No matching {}.rpm artifacts found", aarch).into());
        }

        // If a specific version is requested, filter by that version
        if let Some(version) = version {
            matching.retain(|a| {
                let ver = SemverUtils::extract_version(&a.name);
                ver.is_some_and(|v| v == version)
            });
        }

        // Optional: try to parse semantic version numbers from filenames
        matching.sort_by(|a, b| {
            // extract "1.2.3" style version substrings from name
            let ver_a = SemverUtils::extract_version(&a.name);
            let ver_b = SemverUtils::extract_version(&b.name);

            match (ver_a, ver_b) {
                (Some(va), Some(vb)) => va.cmp(&vb), // descending
                _ => a.name.cmp(&b.name),
            }
        });

        Ok(matching.into_iter().last())
    }
}
