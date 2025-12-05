use std::sync::Arc;

use aws_config::BehaviorVersion;
use aws_sdk_s3::primitives::ByteStream;
use google_gmail1::yup_oauth2::storage::{TokenInfo, TokenStorage};
use tracing::{debug, error, info};

pub struct S3TokenStorage {
    bucket: String,
    key: String,
    s3: Arc<aws_sdk_s3::Client>,
}

impl S3TokenStorage {
    pub async fn new(bucket: impl Into<String>, key: impl Into<String>) -> Self {
        let bucket = bucket.into();
        let key = key.into();
        debug!("S3TokenStorage::new - bucket: {}, key: {}", bucket, key);

        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let s3 = Arc::new(aws_sdk_s3::Client::new(&config));

        Self { bucket, key, s3 }
    }
}

#[async_trait::async_trait]
impl TokenStorage for S3TokenStorage {
    async fn set(&self, _scopes: &[&str], token: TokenInfo) -> anyhow::Result<()> {
        debug!(
            "S3TokenStorage::set - Saving token to s3://{}/{}",
            &self.bucket, &self.key
        );
        let json = serde_json::to_string(&token)?;

        self.s3
            .put_object()
            .bucket(&self.bucket)
            .key(&self.key)
            .body(ByteStream::from(json.into_bytes()))
            .send()
            .await
            .map_err(|e| {
                error!("Error saving credentials to S3: {}", e);
                e
            })?;

        info!("Token saved to S3: s3://{}/{}", &self.bucket, &self.key);

        Ok(())
    }

    async fn get(&self, _scopes: &[&str]) -> Option<TokenInfo> {
        debug!(
            "S3TokenStorage::get - Fetching token from s3://{}/{}",
            &self.bucket, &self.key
        );
        match self
            .s3
            .get_object()
            .bucket(&self.bucket)
            .key(&self.key)
            .send()
            .await
        {
            Ok(output) => {
                debug!("S3TokenStorage::get - Successfully fetched object from S3");
                let data = output
                    .body
                    .collect()
                    .await
                    .map_err(|e| {
                        error!("Error getting credentials from S3: {}", e);
                        e
                    })
                    .ok()?;
                let bytes = data.into_bytes();
                debug!(
                    "S3TokenStorage::get - Token data size: {} bytes",
                    bytes.len()
                );
                match serde_json::from_slice::<TokenInfo>(&bytes) {
                    Ok(token) => {
                        debug!(
                            "S3TokenStorage::get - Successfully parsed token, has refresh_token: {}",
                            token.refresh_token.is_some()
                        );
                        Some(token)
                    }
                    Err(e) => {
                        error!("S3TokenStorage::get - Failed to parse token: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                error!("Error getting credentials from S3: {}", e);
                None
            }
        }
    }
}
