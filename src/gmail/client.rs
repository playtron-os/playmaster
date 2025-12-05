use std::path::PathBuf;

use google_gmail1::{
    Gmail,
    api::Message,
    hyper_rustls::{HttpsConnector, HttpsConnectorBuilder},
    hyper_util::{
        client::legacy::{Client, connect::HttpConnector},
        rt::TokioExecutor,
    },
    yup_oauth2::{
        ApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod,
        authenticator::Authenticator, storage::TokenStorage,
    },
};
use regex::Regex;
use tracing::{debug, error, info};

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
        after_timestamp: Option<i64>,
        timeout_secs: u64,
        poll_interval_secs: u64,
    ) -> ResultWithError<String> {
        debug!("fetch_latest_email_matching_regex called");
        debug!(
            "S3 bucket: {:?}, S3 key_prefix: {:?}",
            self.s3_bucket, self.s3_key_prefix
        );

        //  Auth
        let secret = match self.get_secret().await {
            Ok(s) => {
                debug!("Successfully loaded OAuth client credentials");
                s
            }
            Err(e) => {
                error!("Failed to load OAuth client credentials: {}", e);
                return Err(e);
            }
        };

        let auth = match self.get_flow(secret).await {
            Ok(a) => {
                debug!("Successfully created authenticator");
                a
            }
            Err(e) => {
                error!("Failed to create authenticator: {}", e);
                return Err(e);
            }
        };

        let executor = TokioExecutor::new();
        let https = HttpsConnectorBuilder::new()
            .with_native_roots()?
            .https_only()
            .enable_http1()
            .build();
        let client = Client::builder(executor).build(https);

        let hub = Gmail::new(client, auth);
        let user_id = "me";

        // Build query
        let q = self.build_query(from, subject_contains, after_timestamp);
        debug!("Gmail query: {}", q);

        // Poll for a message
        let msg_id = self
            .search_latest_message(&hub, user_id, &q, timeout_secs, poll_interval_secs)
            .await?;

        debug!("Found message with ID: {}", msg_id);

        // Fetch body
        let body = self.get_email_body(&hub, user_id, &msg_id).await?;
        debug!("Email body length: {} chars", body.len());

        // Extract via regex
        self.extract_code_from_body(&body, regex)
    }

    /// Validates that Gmail credentials are available and working.
    /// Returns Ok(()) if credentials are valid, or an error if authentication is needed.
    pub async fn validate_credentials(&self) -> EmptyResult {
        debug!("Validating Gmail credentials");
        debug!(
            "S3 bucket: {:?}, S3 key_prefix: {:?}",
            self.s3_bucket, self.s3_key_prefix
        );

        // Check if we have a stored token with a refresh token
        if let Some(storage) = self.get_storage().await {
            debug!("Got S3 storage, checking for token");
            let scopes = &["https://www.googleapis.com/auth/gmail.readonly"];
            if let Some(token) = storage.get(scopes).await {
                debug!(
                    "Found token in S3, has refresh_token: {}",
                    token.refresh_token.is_some()
                );
                // If we have a refresh token, credentials are valid (OAuth will handle refresh)
                if token.refresh_token.is_some() {
                    debug!("Gmail refresh token exists, credentials are valid");
                    return Ok(());
                }
            } else {
                debug!("No token found in S3 storage");
            }
        } else {
            debug!("No S3 storage configured");
        }

        Err("Gmail credentials not found or invalid. Please run 'playmaster gmail' to authenticate.".into())
    }

    /// Ensures Gmail credentials are valid, running authentication flow if needed.
    /// This should be called at the start of test runs that require Gmail.
    pub async fn ensure_authenticated(&self) -> EmptyResult {
        match self.validate_credentials().await {
            Ok(()) => {
                info!("Gmail credentials validated successfully");
                Ok(())
            }
            Err(_) => {
                info!("Gmail credentials not found or expired, initiating authentication flow");
                self.generate_refresh_token().await
            }
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

    fn build_query(
        &self,
        from: &str,
        subject_contains: &str,
        after_timestamp: Option<i64>,
    ) -> String {
        let mut q = format!("from:{} subject:{}", from, subject_contains);

        if let Some(ts) = after_timestamp {
            q.push_str(&format!(" after:{}", ts));
        }

        q
    }

    async fn search_latest_message(
        &self,
        hub: &Gmail<HttpsConnector<HttpConnector>>,
        user_id: &str,
        q: &str,
        timeout_secs: u64,
        poll_interval_secs: u64,
    ) -> ResultWithError<String> {
        let start = tokio::time::Instant::now();

        loop {
            if start.elapsed().as_secs() >= timeout_secs {
                return Err(format!("Timed out after {} seconds", timeout_secs).into());
            }

            info!("Searching for email with query: {}", q);

            let resp = hub
                .users()
                .messages_list(user_id)
                .q(q)
                .max_results(1)
                .doit()
                .await?
                .1;

            if let Some(messages) = resp.messages
                && let Some(msg) = messages.first()
                && let Some(id) = &msg.id
            {
                return Ok(id.clone());
            }

            tokio::time::sleep(std::time::Duration::from_secs(poll_interval_secs)).await;
        }
    }

    async fn get_email_body(
        &self,
        hub: &Gmail<HttpsConnector<HttpConnector>>,
        user_id: &str,
        msg_id: &str,
    ) -> ResultWithError<String> {
        let msg: Message = hub
            .users()
            .messages_get(user_id, msg_id)
            .format("full")
            .doit()
            .await?
            .1;

        self.extract_body_from_message(&msg)
    }

    fn extract_code_from_body(&self, body: &str, regex: &Regex) -> ResultWithError<String> {
        if let Some(caps) = regex.captures(body) {
            Ok(caps[1].to_string())
        } else {
            Err("Regex did not match email body".into())
        }
    }

    async fn get_secret(&self) -> ResultWithError<ApplicationSecret> {
        debug!("get_secret: Looking for OAuth client credentials file");
        let credentials = Self::find_credentials_json()?;
        debug!("get_secret: Found credentials at {:?}", credentials);
        Ok(google_gmail1::yup_oauth2::read_application_secret(credentials).await?)
    }

    async fn get_flow(
        &self,
        secret: ApplicationSecret,
    ) -> ResultWithError<Authenticator<HttpsConnector<HttpConnector>>> {
        debug!(
            "get_flow: Creating authenticator, S3 bucket: {:?}",
            self.s3_bucket
        );
        if let Some(storage) = self.get_storage().await {
            debug!("get_flow: Using S3 token storage");
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
            debug!("get_flow: Using local token storage");
            let token_path = DirUtils::config_dir()?;
            let token_path = token_path.join("gmail_token.json");
            debug!("get_flow: Local token path: {:?}", token_path);

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
