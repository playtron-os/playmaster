use std::net::TcpStream;
use std::time::Duration;

use imap::Session;
use native_tls::{TlsConnector, TlsStream};
use regex::Regex;
use tracing::{debug, error, info};

use crate::utils::errors::ResultWithError;

const IMAP_DOMAIN: &str = "imap.gmail.com";
const IMAP_PORT: u16 = 993;

pub struct ImapGmailClient {
    email: String,
    app_password: String,
}

impl ImapGmailClient {
    pub fn new(email: String, app_password: String) -> Self {
        Self {
            email,
            app_password,
        }
    }

    /// Fetches the latest email matching the criteria and extracts a code using the regex.
    pub fn fetch_latest_email_matching_regex(
        &self,
        from: &str,
        subject_contains: &str,
        regex: &Regex,
        after_timestamp: Option<i64>,
        timeout_secs: u64,
        poll_interval_secs: u64,
    ) -> ResultWithError<String> {
        debug!(
            "IMAP: Fetching email from={}, subject_contains={}, after={:?}",
            from, subject_contains, after_timestamp
        );

        let start = std::time::Instant::now();

        loop {
            if start.elapsed().as_secs() >= timeout_secs {
                return Err(
                    format!("Timed out after {} seconds waiting for email", timeout_secs).into(),
                );
            }

            match self.try_fetch_email(from, subject_contains, regex, after_timestamp) {
                Ok(Some(code)) => {
                    info!("IMAP: Successfully extracted code from email");
                    return Ok(code);
                }
                Ok(None) => {
                    debug!("IMAP: No matching email found yet, waiting...");
                }
                Err(e) => {
                    error!("IMAP: Error fetching email: {}", e);
                }
            }

            std::thread::sleep(Duration::from_secs(poll_interval_secs));
        }
    }

    fn try_fetch_email(
        &self,
        from: &str,
        subject_contains: &str,
        regex: &Regex,
        after_timestamp: Option<i64>,
    ) -> ResultWithError<Option<String>> {
        let mut session = self.connect()?;

        // Select INBOX
        session
            .select("INBOX")
            .map_err(|e| format!("Failed to select INBOX: {}", e))?;
        debug!("IMAP: Selected INBOX");

        // Build IMAP search query (SINCE only filters by date, not time)
        let search_query = self.build_search_query(from, subject_contains, after_timestamp);
        debug!("IMAP: Search query: {}", search_query);

        // Search for messages
        let message_ids: Vec<u32> = session
            .search(&search_query)
            .map_err(|e| format!("IMAP search failed: {}", e))?
            .into_iter()
            .collect();

        debug!("IMAP: Found {} matching messages", message_ids.len());

        if message_ids.is_empty() {
            session.logout().ok();
            return Ok(None);
        }

        // Fetch all matching messages with their headers to filter by actual timestamp
        // Sort by ID descending (newest first) and check each one
        let mut sorted_ids = message_ids.clone();
        sorted_ids.sort_by(|a, b| b.cmp(a)); // Descending order

        // Fetch messages in batches, starting from newest
        for msg_id in sorted_ids.iter().take(10) {
            debug!("IMAP: Checking message ID: {}", msg_id);

            // Fetch the message with headers and body
            let messages = session
                .fetch(msg_id.to_string(), "(RFC822 INTERNALDATE)")
                .map_err(|e| format!("Failed to fetch message: {}", e))?;

            for message in messages.iter() {
                // Check the internal date if we have a timestamp filter
                if let Some(after_ts) = after_timestamp
                    && let Some(internal_date) = message.internal_date()
                {
                    let msg_timestamp = internal_date.timestamp();
                    debug!(
                        "IMAP: Message {} internal date timestamp: {} (filter: {})",
                        msg_id, msg_timestamp, after_ts
                    );

                    if msg_timestamp < after_ts {
                        debug!("IMAP: Message {} is older than filter, skipping", msg_id);
                        continue;
                    }
                }

                if let Some(body) = message.body() {
                    let body_str = String::from_utf8_lossy(body);
                    debug!(
                        "IMAP: Message {} body length: {} bytes",
                        msg_id,
                        body_str.len()
                    );

                    // Parse the email to extract the text content
                    let text_content = self.extract_text_from_email(&body_str)?;
                    debug!("IMAP: Extracted text content: {} chars", text_content.len());

                    // Try to match the regex
                    if let Some(caps) = regex.captures(&text_content) {
                        let code = caps[1].to_string();
                        debug!("IMAP: Extracted code: {}", code);
                        session.logout().ok();
                        return Ok(Some(code));
                    }
                }
            }
        }

        session.logout().ok();
        Ok(None)
    }

    fn connect(&self) -> ResultWithError<Session<TlsStream<TcpStream>>> {
        debug!("IMAP: Connecting to {}:{}", IMAP_DOMAIN, IMAP_PORT);

        let tls = TlsConnector::builder()
            .build()
            .map_err(|e| format!("Failed to create TLS connector: {}", e))?;

        let client = imap::connect((IMAP_DOMAIN, IMAP_PORT), IMAP_DOMAIN, &tls)
            .map_err(|e| format!("Failed to connect to IMAP server: {}", e))?;

        debug!("IMAP: Connected, logging in as {}", self.email);

        let session = client
            .login(&self.email, &self.app_password)
            .map_err(|e| format!("IMAP login failed: {:?}", e.0))?;

        debug!("IMAP: Login successful");
        Ok(session)
    }

    fn build_search_query(
        &self,
        from: &str,
        subject_contains: &str,
        after_timestamp: Option<i64>,
    ) -> String {
        let mut query_parts = vec![
            format!("FROM \"{}\"", from),
            format!("SUBJECT \"{}\"", subject_contains),
        ];

        if let Some(ts) = after_timestamp {
            // Convert Unix timestamp to IMAP date format (DD-Mon-YYYY)
            let datetime = chrono::DateTime::from_timestamp(ts, 0).unwrap_or_else(chrono::Utc::now);
            let date_str = datetime.format("%d-%b-%Y").to_string();
            query_parts.push(format!("SINCE {}", date_str));
        }

        query_parts.join(" ")
    }

    fn extract_text_from_email(&self, raw_email: &str) -> ResultWithError<String> {
        // Use mailparse to parse the email
        let parsed = mailparse::parse_mail(raw_email.as_bytes())
            .map_err(|e| format!("Failed to parse email: {}", e))?;

        // Try to get text/plain content first
        if let Some(text) = Self::extract_text_part(&parsed) {
            return Ok(text);
        }

        // Fallback to the body directly
        Ok(parsed.get_body().unwrap_or_default())
    }

    fn extract_text_part(mail: &mailparse::ParsedMail) -> Option<String> {
        // Check if this part is text/plain
        if let Some(content_type) = mail
            .headers
            .iter()
            .find(|h| h.get_key().eq_ignore_ascii_case("Content-Type"))
        {
            let ct = content_type.get_value();
            if ct.starts_with("text/plain") {
                return mail.get_body().ok();
            }
        }

        // Check subparts (for multipart emails)
        for subpart in &mail.subparts {
            if let Some(text) = Self::extract_text_part(subpart) {
                return Some(text);
            }
        }

        // If no text/plain found, try to get any body
        if mail.subparts.is_empty() {
            return mail.get_body().ok();
        }

        None
    }

    /// Validates that the IMAP credentials are working by attempting to connect.
    pub fn validate_credentials(&self) -> ResultWithError<()> {
        debug!("IMAP: Validating credentials for {}", self.email);
        let mut session = self.connect()?;
        session.logout().ok();
        info!("IMAP: Credentials validated successfully");
        Ok(())
    }
}
