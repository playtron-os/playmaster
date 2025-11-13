use std::path::PathBuf;

use google_gmail1::{
    hyper_rustls::HttpsConnector,
    hyper_util::client::legacy::connect::HttpConnector,
    yup_oauth2::{
        ApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod,
        authenticator::Authenticator,
    },
};
use tracing::{debug, info};

use crate::{
    gmail::s3_storage::S3TokenStorage,
    utils::{
        dir::DirUtils,
        errors::{EmptyResult, ResultWithError},
    },
};

pub struct GmailClient {
    pub s3_bucket: Option<String>,
    pub s3_key_prefix: Option<String>,
}

impl GmailClient {
    pub fn new(s3_bucket: Option<String>, s3_key_prefix: Option<String>) -> Self {
        GmailClient {
            s3_bucket,
            s3_key_prefix,
        }
    }

    pub async fn generate_refresh_token(&self) -> EmptyResult {
        let secret = self.get_secret().await?;
        let auth = self.get_flow(secret).await?;

        let token = auth
            .token(&["https://www.googleapis.com/auth/gmail.readonly"])
            .await?;
        let token = token.token();

        info!("ACCESS TOKEN: {:?}", token);

        Ok(())
    }

    async fn get_secret(&self) -> ResultWithError<ApplicationSecret> {
        let credentials = Self::find_credentials_json()?;
        Ok(google_gmail1::yup_oauth2::read_application_secret(credentials).await?)
    }

    async fn get_flow(
        &self,
        secret: ApplicationSecret,
    ) -> ResultWithError<Authenticator<HttpsConnector<HttpConnector>>> {
        if let Some(storage) = self.get_storage().await {
            Ok(
                InstalledFlowAuthenticator::builder(
                    secret,
                    InstalledFlowReturnMethod::HTTPRedirect,
                )
                .with_storage(Box::new(storage))
                .build()
                .await?,
            )
        } else {
            let token_path = DirUtils::config_dir()?;
            let token_path = token_path.join("gmail_token.json");

            Ok(
                InstalledFlowAuthenticator::builder(
                    secret,
                    InstalledFlowReturnMethod::HTTPRedirect,
                )
                .persist_tokens_to_disk(token_path.clone())
                .build()
                .await?,
            )
        }
    }

    async fn get_storage(&self) -> Option<S3TokenStorage> {
        if let (Some(bucket), Some(prefix)) = (&self.s3_bucket, &self.s3_key_prefix) {
            let key = format!("{}/credentials.json", prefix.trim_end_matches('/'));
            Some(S3TokenStorage::new(bucket.clone(), key).await)
        } else {
            None
        }
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
