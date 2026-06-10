use std::time::Duration;

use crate::{
    ApiResponse, DEFAULT_TIMEOUT, Error, ParseFailureFallback, ParseMode, SendMessageRequest,
    SendOptions, SentMessage, TelegramMessage, fallback_plain_message,
};

/// Async Telegram `sendMessage` client.
#[derive(Clone)]
pub struct TelegramBot {
    client: reqwest::Client,
    base_url: String,
    token: String,
    default_chat_id: String,
    timeout: Duration,
}

impl TelegramBot {
    pub fn new(token: impl Into<String>, default_chat_id: impl Into<String>) -> Self {
        Self::with_client(reqwest::Client::new(), token, default_chat_id)
    }

    pub fn with_client(
        client: reqwest::Client,
        token: impl Into<String>,
        default_chat_id: impl Into<String>,
    ) -> Self {
        Self {
            client,
            base_url: "https://api.telegram.org".to_string(),
            token: token.into(),
            default_chat_id: default_chat_id.into(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn send(&self, message: &TelegramMessage) -> Result<SentMessage, Error> {
        self.send_with_options(message, SendOptions::default())
            .await
    }

    pub async fn send_to(
        &self,
        chat_id: impl Into<String>,
        message: &TelegramMessage,
    ) -> Result<SentMessage, Error> {
        self.send_with_options(
            message,
            SendOptions {
                chat_id: Some(chat_id.into()),
                ..SendOptions::default()
            },
        )
        .await
    }

    pub async fn send_with_options(
        &self,
        message: &TelegramMessage,
        options: SendOptions,
    ) -> Result<SentMessage, Error> {
        let result = self.send_once(message, &options).await;

        if should_retry_plain(message, &options, &result) {
            let fallback = fallback_plain_message(message);
            let fallback_options = SendOptions {
                parse_failure_fallback: ParseFailureFallback::None,
                ..options
            };

            self.send_once(&fallback, &fallback_options).await
        } else {
            result
        }
    }

    async fn send_once(
        &self,
        message: &TelegramMessage,
        options: &SendOptions,
    ) -> Result<SentMessage, Error> {
        let chat_id = options.chat_id.as_deref().unwrap_or(&self.default_chat_id);
        let request = SendMessageRequest::new(chat_id, message, options);
        let response = self
            .client
            .post(self.api_url("sendMessage"))
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(Error::Telegram(body));
        }

        let parsed: ApiResponse<SentMessage> = serde_json::from_str(&body)?;

        if parsed.ok {
            parsed
                .result
                .ok_or_else(|| Error::Telegram("sendMessage returned no result".to_string()))
        } else {
            Err(Error::Telegram(
                parsed
                    .description
                    .unwrap_or_else(|| "unknown error".to_string()),
            ))
        }
    }

    fn api_url(&self, method: &str) -> String {
        format!("{}/bot{}/{}", self.base_url, self.token, method)
    }
}

fn should_retry_plain(
    message: &TelegramMessage,
    options: &SendOptions,
    result: &Result<SentMessage, Error>,
) -> bool {
    result.is_err()
        && message.parse_mode() != ParseMode::Plain
        && options.parse_failure_fallback == ParseFailureFallback::PlainText
}
