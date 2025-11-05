use std::collections::HashMap;

use aws_config::BehaviorVersion;
use aws_sdk_s3::{primitives::ByteStream, types::ObjectCannedAcl};
use tokio::runtime::Runtime;
use tracing::{debug, error, info};

use crate::{
    hooks::iface::{Hook, HookContext, HookType},
    models::{
        app_state::{AppState, Results},
        config::{S3Config, WebhookConfig},
    },
    utils::{
        errors::{EmptyResult, ResultTrait, ResultWithError},
        variables::VariablesUtils,
    },
};

/// Hook to handle reports post test run.
pub struct HookResults {
    config: WebhookConfig,
}

impl Hook for HookResults {
    fn get_type(&self) -> HookType {
        HookType::Finished
    }

    fn continue_on_error(&self) -> bool {
        true
    }

    fn run(&self, ctx: &HookContext<'_, AppState>) -> EmptyResult {
        let results = ctx.get_results()?;
        self.call_webhook(results)?;
        Ok(())
    }
}

impl HookResults {
    pub fn new(config: WebhookConfig) -> Self {
        HookResults { config }
    }

    fn call_webhook(&self, results: Results) -> EmptyResult {
        if self.config.url.is_empty() {
            info!("No webhook URL configured, skipping webhook call.");
            return Ok(());
        }

        info!("Calling webhook {}...", self.config.url);

        let client = reqwest::blocking::Client::new();
        let logs_url = if let Some(s3_config) = self.config.s3_config.as_ref() {
            match self.upload_logs_to_s3(&results, s3_config) {
                Ok(url) => url,
                Err(e) => {
                    error!("Failed to upload logs to S3: {}", e);
                    "".to_owned()
                }
            }
        } else {
            "".to_owned()
        };
        self.send_message(&client, &results, &logs_url)?;

        info!("Webhook called successfully.");

        Ok(())
    }

    fn send_message(
        &self,
        client: &reqwest::blocking::Client,
        results: &Results,
        logs_url: &str,
    ) -> EmptyResult {
        let message = self.get_message(results, logs_url);
        debug!("Webhook message: {}", message);

        let payload =
            serde_json::json!({ "text": message, "results": results, "logs_url": logs_url });
        let res = client
            .post(&self.config.url)
            .json(&payload)
            .send()
            .auto_err("Failed to send webhook message: {}")?;

        if !res.status().is_success() {
            if self.config.ignore_error {
                info!(
                    "Webhook API returned error status: {}, but ignoring as per configuration.",
                    res.status()
                );
                return Ok(());
            }

            return Err(format!("Webhook API returned error status: {}", res.status()).into());
        }

        Ok(())
    }

    fn get_message(&self, results: &Results, logs_url: &str) -> String {
        let errors = if results.error.is_empty() {
            "".to_owned()
        } else {
            results.error.join("\n")
        };

        if !self.config.message_template.is_empty() {
            let mut data = serde_json::to_value(results)
                .unwrap_or_default()
                .as_object()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned()))
                .collect::<HashMap<_, _>>();
            data.insert("logs_url".to_owned(), logs_url.to_owned());
            data.insert("errors".to_owned(), errors);

            data.insert("passed".to_owned(), results.passed.to_string());
            data.insert("failed".to_owned(), results.failed.to_string());
            data.insert("total".to_owned(), results.total.to_string());

            let status = if results.failed > 0 || !results.error.is_empty() {
                "Failed"
            } else {
                "Passed"
            };
            let status_icon = if status == "Passed" { "✅" } else { "❌" };
            data.insert("status".to_owned(), status.to_owned());
            data.insert("status_icon".to_owned(), status_icon.to_owned());

            debug!("Webhook message data: {:?}", data);

            VariablesUtils::replace_vars(&self.config.message_template, &data, None)
        } else {
            format!(
                "Test Run Completed:\n✅ Passed: {}\n❌ Failed: {}\n📋 Total: {}\nStart Time: {}\nEnd Time: {}\nLogs: {}\nErrors: {}",
                results.passed,
                results.failed,
                results.total,
                results.start_time,
                results.end_time,
                logs_url,
                errors
            )
        }
    }

    fn upload_logs_to_s3(
        &self,
        results: &Results,
        s3_config: &S3Config,
    ) -> ResultWithError<String> {
        if results.full_log.is_empty() {
            debug!("No logs to upload to S3.");
            return Ok("".to_owned());
        }

        let rt = Runtime::new().auto_err("Failed to create runtime")?;
        let key_prefix = s3_config.key_prefix.clone();
        let bucket = s3_config.bucket.clone();

        rt.block_on(async {
            let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
            let s3 = aws_sdk_s3::Client::new(&config);

            let key = format!("{}results_{}.txt", key_prefix, results.start_time).replace(" ", "_");
            info!("Uploading logs to S3 to path: s3://{}/{}", bucket, key);

            // Upload from the in-memory string
            s3.put_object()
                .bucket(&bucket)
                .key(&key)
                .acl(ObjectCannedAcl::PublicRead)
                .body(ByteStream::from(results.full_log.clone().into_bytes()))
                .send()
                .await
                .auto_err("Failed to upload logs to S3")?;

            let s3_url = format!("https://{}.s3.amazonaws.com/{}", bucket, key);
            info!("Logs uploaded to S3 successfully: {}", s3_url);

            Ok(s3_url)
        })
    }
}
