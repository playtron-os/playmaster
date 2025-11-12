use std::path::PathBuf;

use google_gmail1::yup_oauth2::{
    ApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod,
};
use tracing::debug;

use crate::utils::{
    dir::DirUtils,
    errors::{EmptyResult, ResultWithError},
};

pub struct GmailClient;

impl GmailClient {
    pub fn new() -> Self {
        GmailClient {}
    }

    pub async fn generate_refresh_token(&self) -> EmptyResult {
        let credentials = Self::find_credentials_json()?;
        let secret: ApplicationSecret =
            google_gmail1::yup_oauth2::read_application_secret(credentials).await?;

        let token_path = DirUtils::config_dir()?;
        let token_path = token_path.join("gmail_token.json");

        println!("Using token storage at: {}", token_path.display());

        let auth = InstalledFlowAuthenticator::builder(
            secret,
            InstalledFlowReturnMethod::Interactive, // opens browser
        )
        .persist_tokens_to_disk(token_path.clone())
        .build()
        .await?;

        let token = auth
            .token(&["https://www.googleapis.com/auth/gmail.readonly"])
            .await?;
        let token = token.token();

        println!("ACCESS TOKEN: {:?}", token);

        Ok(())
    }

    fn find_credentials_json() -> ResultWithError<PathBuf> {
        let mut candidates = Vec::new();

        candidates.push(PathBuf::from("credentials.json"));

        if let Some(home) = dirs::home_dir() {
            candidates.push(home.join("credentials.json"));
        }

        if let Ok(dir) = DirUtils::config_dir() {
            candidates.push(dir.join("credentials.json"));
        }

        for p in candidates {
            debug!("Checking for credentials.json at: {:?}", p);

            if p.exists() && p.is_file() {
                return Ok(p);
            }
        }

        Err("credentials.json not found".into())
    }
}
