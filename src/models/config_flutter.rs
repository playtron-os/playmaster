use crate::models::config::{Config, Dependency, InstallSpec};

impl Config {
    pub fn add_flutter_defaults(&mut self) {
        if !self.dependencies.iter().any(|d| d.name == "git") {
            let git_dep = Dependency {
                name: "git".into(),
                min_version: "2.0.0".into(),
                version_command: "git --version | awk '{print $3}'".into(),
                install: Some(InstallSpec {
                    tool: "git".into(), // Fedora / RHEL package name
                    version: None,
                    bin_path: None,
                    setup: None,
                    source: None,
                }),
            };
            self.dependencies.push(git_dep);
        }

        if !self.dependencies.iter().any(|d| d.name == "flutter") {
            let flutter_dep = Dependency {
                name: "flutter".into(),
                min_version: "3.29.2".into(),
                version_command: "flutter --version | head -n 1 | awk '{print $2}'".into(),
                install: Some(InstallSpec {
                    tool: "flutter".into(),
                    version: Some("3.29.2".into()),
                    bin_path: Some("flutter/bin".into()),
                    setup: Some("flutter --version || true".into()),
                    source: Some(crate::models::config::InstallSource::Url {
                        url: "https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_{{version}}-stable.tar.xz".into()
                    }),
                }),
            };
            self.dependencies.push(flutter_dep);
        }
    }
}
