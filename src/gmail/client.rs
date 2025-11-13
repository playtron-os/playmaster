use std::path::PathBuf;

use google_gmail1::{
    Gmail,
    api::{ListMessagesResponse, Message},
    hyper_rustls::{HttpsConnector, HttpsConnectorBuilder},
    hyper_util::{
        client::legacy::{Client, connect::HttpConnector},
        rt::TokioExecutor,
    },
    yup_oauth2::{
        ApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod,
        authenticator::Authenticator,
    },
};
use regex::Regex;
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

    pub async fn fetch_latest_email_matching_regex(
        &self,
        from: &str,
        subject_contains: &str,
        regex: &Regex,
    ) -> ResultWithError<String> {
        // Auth
        let secret = self.get_secret().await?;
        let auth = self.get_flow(secret).await?;

        // Gmail API client
        let executor = TokioExecutor::new();
        let https = HttpsConnectorBuilder::new()
            .with_native_roots()?
            .https_only()
            .enable_http1()
            .build();
        let client = Client::builder(executor).build(https);
        let hub = Gmail::new(client, auth);

        let user_id = "me";
        let q = format!("from:{} subject:{}", from, subject_contains);

        // List matching messages
        let resp: ListMessagesResponse = hub
            .users()
            .messages_list(user_id)
            .q(&q)
            .max_results(1)
            .doit()
            .await?
            .1; // second item is the response

        let Some(messages) = resp.messages else {
            return Err("No matching MFA emails found".into());
        };

        let latest_msg_id = messages[0].id.clone().unwrap();

        // Fetch the full email, including payload
        let full_msg: Message = hub
            .users()
            .messages_get(user_id, &latest_msg_id)
            .format("full")
            .doit()
            .await?
            .1;

        // Extract plain text from message payload
        let body = self.extract_body_from_message(&full_msg)?;

        // Extract text from body using regex
        if let Some(caps) = regex.captures(&body) {
            return Ok(caps[1].to_string());
        }

        Err("Could not find text in email body".into())
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

    fn extract_body_from_message(
        &self,
        msg: &google_gmail1::api::Message,
    ) -> ResultWithError<String> {
        let payload = msg
            .payload
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing email payload"))?;

        // If body is directly available
        if let Some(body) = &payload.body
            && let Some(data) = &body.data
        {
            debug!("Found body in payload, decoding... {data:?}");

            let decoded = self.decode_gmail_body(data)?;
            return Ok(decoded);
        }

        // Otherwise search parts
        if let Some(parts) = &payload.parts {
            for part in parts {
                if let Some(mime_type) = &part.mime_type {
                    debug!("Searching email part for body... {mime_type:?}");

                    // Prefer text/plain, fall back to text/html
                    if (mime_type == "text/plain" || mime_type == "text/html")
                        && let Some(body) = &part.body
                        && let Some(data) = &body.data
                    {
                        debug!("Email part body found... {data:?}");

                        let decoded = self.decode_gmail_body(data)?;
                        return Ok(decoded);
                    }
                }
            }
        }

        Err("Could not extract email body".into())
    }

    fn decode_gmail_body(&self, data: &[u8]) -> ResultWithError<String> {
        use base64::Engine;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;

        let s = std::str::from_utf8(data)?.trim();

        // Case 1: looks like HTML or text (NOT base64)
        if s.contains("<") || s.contains(">") || s.contains(":") {
            return Ok(s.to_string());
        }

        // Case 2: quoted-printable (soft breaks =\n etc)
        if s.contains("=\n") || s.contains("=\r\n") {
            let decoded = quoted_printable::decode(s, quoted_printable::ParseMode::Robust)?;
            return Ok(String::from_utf8_lossy(&decoded).to_string());
        }

        // Case 3: attempt base64url decoding
        let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();

        let decoded = URL_SAFE_NO_PAD.decode(cleaned.as_bytes())?;

        Ok(String::from_utf8_lossy(&decoded).to_string())
    }
}
