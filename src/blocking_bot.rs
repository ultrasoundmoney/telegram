use std::time::Duration;

use crate::{
    ApiResponse, DEFAULT_TIMEOUT, Error, ParseFailureFallback, ParseMode, SendMessageRequest,
    SendOptions, SentMessage, TelegramMessage, fallback_plain_message,
};
use tracing::{debug, error, info, instrument, warn};

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
        self.send_with_options_inner(message, options)
    }

    #[instrument(
        name = "telegram.send_message",
        skip_all,
        fields(
            telegram.transport = "blocking",
            telegram.parse_mode = ?message.parse_mode(),
            telegram.text_chars = message.text().chars().count(),
            telegram.chat_override = options.chat_id.is_some(),
            telegram.message_thread_id = options.message_thread_id,
            telegram.has_reply_markup = options.reply_markup.is_some(),
        )
    )]
    fn send_with_options_inner(
        &self,
        message: &TelegramMessage,
        options: SendOptions,
    ) -> Result<SentMessage, Error> {
        info!("sending telegram message");
        let result = self.send_once(message, &options);

        if should_retry_plain(message, &options, &result) {
            if let Err(error) = &result {
                warn!(
                    telegram.error_kind = error.kind(),
                    "telegram send failed; retrying without parse mode"
                );
            }

            let fallback = fallback_plain_message(message);
            let fallback_options = SendOptions {
                parse_failure_fallback: ParseFailureFallback::None,
                ..options
            };

            let fallback_result = self.send_once(&fallback, &fallback_options);
            log_send_result(&fallback_result, true);
            fallback_result
        } else {
            log_send_result(&result, false);
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
        debug!("posting telegram sendMessage request");
        let mut response = self
            .agent
            .post(&self.api_url("sendMessage"))
            .send_json(&request)?;
        debug!(
            http.status = response.status().as_u16(),
            "received telegram response"
        );
        let parsed: ApiResponse<SentMessage> = response.body_mut().read_json()?;

        if parsed.ok {
            parsed.result.ok_or_else(|| {
                warn!("telegram api response did not include a message result");
                Error::Telegram("sendMessage returned no result".to_string())
            })
        } else {
            let description = parsed
                .description
                .unwrap_or_else(|| "unknown error".to_string());
            warn!(telegram.error = %description, "telegram api returned an error");
            Err(Error::Telegram(description))
        }
    }

    fn api_url(&self, method: &str) -> String {
        format!("{}/bot{}/{}", self.base_url, self.token, method)
    }
}

fn log_send_result(result: &Result<SentMessage, Error>, used_plain_fallback: bool) {
    match result {
        Ok(message) => {
            info!(
                telegram.message_id = message.message_id,
                telegram.used_plain_fallback = used_plain_fallback,
                "sent telegram message"
            );
        }
        Err(error) => {
            error!(
                telegram.error_kind = error.kind(),
                telegram.used_plain_fallback = used_plain_fallback,
                "failed to send telegram message"
            );
        }
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
