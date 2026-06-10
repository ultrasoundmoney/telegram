use std::time::Duration;

use crate::{
    ApiResponse, DEFAULT_TIMEOUT, Error, ParseFailureFallback, ParseMode, SendMessageRequest,
    SendOptions, SentMessage, TelegramMessage, fallback_plain_message,
};

/// Blocking Telegram `sendMessage` client.
#[derive(Clone)]
pub struct BlockingTelegramBot {
    agent: ureq::Agent,
    base_url: String,
    token: String,
    default_chat_id: String,
}

impl BlockingTelegramBot {
    pub fn new(token: impl Into<String>, default_chat_id: impl Into<String>) -> Self {
        Self::with_agent(
            ureq::Agent::config_builder()
                .timeout_global(Some(DEFAULT_TIMEOUT))
                .build()
                .into(),
            token,
            default_chat_id,
        )
    }

    pub fn with_agent(
        agent: ureq::Agent,
        token: impl Into<String>,
        default_chat_id: impl Into<String>,
    ) -> Self {
        Self {
            agent,
            base_url: "https://api.telegram.org".to_string(),
            token: token.into(),
            default_chat_id: default_chat_id.into(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_timeout(self, timeout: Duration) -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(timeout))
                .build()
                .into(),
            ..self
        }
    }

    pub fn send(&self, message: &TelegramMessage) -> Result<SentMessage, Error> {
        self.send_with_options(message, SendOptions::default())
    }

    pub fn send_to(
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
    }

    pub fn send_with_options(
        &self,
        message: &TelegramMessage,
        options: SendOptions,
    ) -> Result<SentMessage, Error> {
        let result = self.send_once(message, &options);

        if should_retry_plain(message, &options, &result) {
            let fallback = fallback_plain_message(message);
            let fallback_options = SendOptions {
                parse_failure_fallback: ParseFailureFallback::None,
                ..options
            };

            self.send_once(&fallback, &fallback_options)
        } else {
            result
        }
    }

    fn send_once(
        &self,
        message: &TelegramMessage,
        options: &SendOptions,
    ) -> Result<SentMessage, Error> {
        let chat_id = options.chat_id.as_deref().unwrap_or(&self.default_chat_id);
        let request = SendMessageRequest::new(chat_id, message, options);
        let mut response = self
            .agent
            .post(&self.api_url("sendMessage"))
            .send_json(&request)?;
        let parsed: ApiResponse<SentMessage> = response.body_mut().read_json()?;

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
