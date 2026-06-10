use std::{fmt, time::Duration};

use serde::{Deserialize, Serialize};
use telegram_sanitize::{SEND_MESSAGE_TEXT_MAX_CHARS, html, markdown_v2, plain_text};

use crate::Error;

pub const CALLBACK_DATA_MAX_BYTES: usize = 64;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(6);
const DEFAULT_VALUE_TEXT_MAX_CHARS: usize = SEND_MESSAGE_TEXT_MAX_CHARS / 4;
const TRUNCATION_MARKER: &str = "\n[truncated]";

/// Per-field budget for dynamic key/value and multiline values.
///
/// The default budget keeps one very large value from pushing later fields out
/// of the message. `Unlimited` is intended for values the caller already
/// normalized or intentionally allows to dominate whole-message truncation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueBudget {
    /// Use the crate's conservative per-value budget.
    Default,
    /// Limit this value to the given number of raw text characters.
    Chars(usize),
    /// Leave this value unbounded and rely only on whole-message truncation.
    Unlimited,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseMode {
    Plain,
    MarkdownV2,
    Html,
}

impl ParseMode {
    pub fn as_telegram_parse_mode(self) -> Option<&'static str> {
        match self {
            Self::Plain => None,
            Self::MarkdownV2 => Some("MarkdownV2"),
            Self::Html => Some("HTML"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TelegramMessage {
    text: String,
    parse_mode: ParseMode,
}

impl TelegramMessage {
    pub fn plain(text: impl Into<String>) -> Self {
        Self::from_text(ParseMode::Plain, text.into())
    }

    pub fn from_text(parse_mode: ParseMode, text: String) -> Self {
        let text = truncate_to_limit(&text);
        Self { text, parse_mode }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn parse_mode(&self) -> ParseMode {
        self.parse_mode
    }

    pub fn parse_mode_parameter(&self) -> Option<&'static str> {
        self.parse_mode.as_telegram_parse_mode()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MessageBuilder {
    parse_mode: ParseMode,
    blocks: Vec<Block>,
}

impl MessageBuilder {
    pub fn new(parse_mode: ParseMode) -> Self {
        Self {
            parse_mode,
            blocks: Vec::new(),
        }
    }

    pub fn plain() -> Self {
        Self::new(ParseMode::Plain)
    }

    pub fn markdown_v2() -> Self {
        Self::new(ParseMode::MarkdownV2)
    }

    pub fn html() -> Self {
        Self::new(ParseMode::Html)
    }

    pub fn line(mut self, text: impl fmt::Display) -> Self {
        self.blocks
            .push(Block::Line(TextStyle::Normal, text.to_string()));
        self
    }

    pub fn bold_line(mut self, text: impl fmt::Display) -> Self {
        self.blocks
            .push(Block::Line(TextStyle::Bold, text.to_string()));
        self
    }

    pub fn kv(self, key: impl fmt::Display, value: impl fmt::Display) -> Self {
        self.kv_with_budget(key, value, ValueBudget::Default)
    }

    pub fn kv_with_budget(
        mut self,
        key: impl fmt::Display,
        value: impl fmt::Display,
        budget: ValueBudget,
    ) -> Self {
        self.blocks.push(Block::KeyValue {
            key: key.to_string(),
            value: Value::Text {
                value: value.to_string(),
                budget,
            },
        });
        self
    }

    pub fn kv_code(self, key: impl fmt::Display, value: impl fmt::Display) -> Self {
        self.kv_code_with_budget(key, value, ValueBudget::Default)
    }

    pub fn kv_code_with_budget(
        mut self,
        key: impl fmt::Display,
        value: impl fmt::Display,
        budget: ValueBudget,
    ) -> Self {
        self.blocks.push(Block::KeyValue {
            key: key.to_string(),
            value: Value::InlineCode {
                value: value.to_string(),
                budget,
            },
        });
        self
    }

    pub fn error(self, key: impl fmt::Display, value: impl fmt::Display) -> Self {
        self.error_with_budget(key, value, ValueBudget::Default)
    }

    pub fn error_with_budget(
        mut self,
        key: impl fmt::Display,
        value: impl fmt::Display,
        budget: ValueBudget,
    ) -> Self {
        self.blocks.push(Block::MultilineValue {
            key: key.to_string(),
            value: value.to_string(),
            budget,
        });
        self
    }

    pub fn code_block(self, key: impl fmt::Display, value: impl fmt::Display) -> Self {
        self.error(key, value)
    }

    pub fn code_block_with_budget(
        self,
        key: impl fmt::Display,
        value: impl fmt::Display,
        budget: ValueBudget,
    ) -> Self {
        self.error_with_budget(key, value, budget)
    }

    pub fn build(self) -> TelegramMessage {
        let rendered = self.render(self.parse_mode);

        if char_count(&rendered) <= SEND_MESSAGE_TEXT_MAX_CHARS {
            TelegramMessage {
                text: rendered,
                parse_mode: self.parse_mode,
            }
        } else {
            TelegramMessage {
                text: truncate_to_limit(&self.render(ParseMode::Plain)),
                parse_mode: ParseMode::Plain,
            }
        }
    }

    pub fn try_build_untruncated(self) -> Result<TelegramMessage, Error> {
        let rendered = self.render(self.parse_mode);
        let chars = char_count(&rendered);

        if chars <= SEND_MESSAGE_TEXT_MAX_CHARS {
            Ok(TelegramMessage {
                text: rendered,
                parse_mode: self.parse_mode,
            })
        } else {
            Err(Error::MessageTooLong {
                chars,
                limit: SEND_MESSAGE_TEXT_MAX_CHARS,
            })
        }
    }

    fn render(&self, parse_mode: ParseMode) -> String {
        self.blocks
            .iter()
            .map(|block| block.render(parse_mode))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Block {
    Line(TextStyle, String),
    KeyValue {
        key: String,
        value: Value,
    },
    MultilineValue {
        key: String,
        value: String,
        budget: ValueBudget,
    },
}

impl Block {
    fn render(&self, parse_mode: ParseMode) -> String {
        match self {
            Self::Line(style, text) => style.render(text, parse_mode),
            Self::KeyValue { key, value } => {
                format!(
                    "{}: {}",
                    render_text(key, parse_mode),
                    value.render(parse_mode)
                )
            }
            Self::MultilineValue { key, value, budget } => match parse_mode {
                ParseMode::Plain => {
                    format!("{}:\n{}", plain_text(key), plain_text(&budget.apply(value)))
                }
                ParseMode::MarkdownV2 => {
                    format!(
                        "{}:\n{}",
                        markdown_v2::text(key),
                        markdown_v2::code_block(&budget.apply(value))
                    )
                }
                ParseMode::Html => {
                    format!("{}:\n{}", html::text(key), html::pre(&budget.apply(value)))
                }
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TextStyle {
    Normal,
    Bold,
}

impl TextStyle {
    fn render(self, text: &str, parse_mode: ParseMode) -> String {
        match (self, parse_mode) {
            (Self::Normal, _) => render_text(text, parse_mode),
            (Self::Bold, ParseMode::Plain) => plain_text(text),
            (Self::Bold, ParseMode::MarkdownV2) => format!("*{}*", markdown_v2::text(text)),
            (Self::Bold, ParseMode::Html) => format!("<b>{}</b>", html::text(text)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Value {
    Text { value: String, budget: ValueBudget },
    InlineCode { value: String, budget: ValueBudget },
}

impl Value {
    fn render(&self, parse_mode: ParseMode) -> String {
        match self {
            Self::Text { value, budget } => render_text(&budget.apply(value), parse_mode),
            Self::InlineCode { value, budget } => match parse_mode {
                ParseMode::Plain => plain_text(&budget.apply(value)),
                ParseMode::MarkdownV2 => markdown_v2::inline_code(&budget.apply(value)),
                ParseMode::Html => html::code(&budget.apply(value)),
            },
        }
    }
}

impl ValueBudget {
    fn apply(self, value: &str) -> String {
        match self {
            Self::Default => truncate_to_char_limit(value, DEFAULT_VALUE_TEXT_MAX_CHARS),
            Self::Chars(limit) => truncate_to_char_limit(value, limit),
            Self::Unlimited => value.to_string(),
        }
    }
}

fn render_text(input: &str, parse_mode: ParseMode) -> String {
    match parse_mode {
        ParseMode::Plain => plain_text(input),
        ParseMode::MarkdownV2 => markdown_v2::text(input),
        ParseMode::Html => html::text(input),
    }
}

fn truncate_to_limit(input: &str) -> String {
    truncate_to_char_limit(input, SEND_MESSAGE_TEXT_MAX_CHARS)
}

fn truncate_to_char_limit(input: &str, limit: usize) -> String {
    if char_count(input) <= limit {
        input.to_string()
    } else {
        let marker_chars = char_count(TRUNCATION_MARKER);
        let keep = limit.saturating_sub(marker_chars);

        input
            .chars()
            .take(keep)
            .chain(TRUNCATION_MARKER.chars())
            .take(limit)
            .collect()
    }
}

fn char_count(input: &str) -> usize {
    input.chars().count()
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InlineKeyboardMarkup {
    pub inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

impl InlineKeyboardMarkup {
    pub fn single(button: InlineKeyboardButton) -> Self {
        Self {
            inline_keyboard: vec![vec![button]],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InlineKeyboardButton {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_data: Option<String>,
}

impl InlineKeyboardButton {
    pub fn url(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            url: Some(url.into()),
            callback_data: None,
        }
    }

    pub fn callback(text: impl Into<String>, data: impl Into<String>) -> Result<Self, Error> {
        let data = data.into();
        let bytes = data.len();

        if bytes <= CALLBACK_DATA_MAX_BYTES {
            Ok(Self {
                text: text.into(),
                url: None,
                callback_data: Some(data),
            })
        } else {
            Err(Error::CallbackDataTooLong { bytes })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseFailureFallback {
    None,
    PlainText,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SendOptions {
    pub chat_id: Option<String>,
    pub disable_web_page_preview: bool,
    pub message_thread_id: Option<i64>,
    pub reply_markup: Option<InlineKeyboardMarkup>,
    pub parse_failure_fallback: ParseFailureFallback,
}

impl Default for SendOptions {
    fn default() -> Self {
        Self {
            chat_id: None,
            disable_web_page_preview: true,
            message_thread_id: None,
            reply_markup: None,
            parse_failure_fallback: ParseFailureFallback::PlainText,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SentMessage {
    pub message_id: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct SendMessageRequest<'a> {
    chat_id: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_mode: Option<&'static str>,
    disable_web_page_preview: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_thread_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_markup: Option<&'a InlineKeyboardMarkup>,
}

impl<'a> SendMessageRequest<'a> {
    pub(crate) fn new(
        chat_id: &'a str,
        message: &'a TelegramMessage,
        options: &'a SendOptions,
    ) -> Self {
        Self {
            chat_id,
            text: message.text(),
            parse_mode: message.parse_mode_parameter(),
            disable_web_page_preview: options.disable_web_page_preview,
            message_thread_id: options.message_thread_id,
            reply_markup: options.reply_markup.as_ref(),
        }
    }
}

pub(crate) fn fallback_plain_message(message: &TelegramMessage) -> TelegramMessage {
    TelegramMessage::plain(message.text())
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub(crate) struct ApiResponse<T> {
    pub ok: bool,
    pub result: Option<T>,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_builder_renders_key_values_and_error() {
        let message = MessageBuilder::plain()
            .line("cielago alert: relay submissions disabled")
            .kv("label", "rbx-prod-mainnet")
            .error("error", "db\nunavailable")
            .build();

        assert_eq!(message.parse_mode(), ParseMode::Plain);
        assert_eq!(
            message.text(),
            "cielago alert: relay submissions disabled\nlabel: rbx-prod-mainnet\nerror:\ndb\nunavailable"
        );
    }

    #[test]
    fn markdown_builder_escapes_and_wraps_code() {
        let message = MessageBuilder::markdown_v2()
            .bold_line("builder demoted")
            .kv_code("slot", 12345)
            .error("error", "bad `root` \\ upstream")
            .build();

        assert_eq!(message.parse_mode(), ParseMode::MarkdownV2);
        assert_eq!(
            message.text(),
            "*builder demoted*\nslot: `12345`\nerror:\n```\nbad \\`root\\` \\\\ upstream\n```"
        );
    }

    #[test]
    fn html_builder_escapes_and_wraps_code() {
        let message = MessageBuilder::html()
            .bold_line("builder <demoted>")
            .kv_code("builder_id", "beaver <prod>")
            .error("error", "invalid <root> & upstream")
            .build();

        assert_eq!(message.parse_mode(), ParseMode::Html);
        assert_eq!(
            message.text(),
            "<b>builder &lt;demoted&gt;</b>\nbuilder_id: <code>beaver &lt;prod&gt;</code>\nerror:\n<pre>invalid &lt;root&gt; &amp; upstream</pre>"
        );
    }

    #[test]
    fn long_plain_message_is_truncated_to_limit() {
        let message = TelegramMessage::plain("x".repeat(SEND_MESSAGE_TEXT_MAX_CHARS + 10));

        assert_eq!(char_count(message.text()), SEND_MESSAGE_TEXT_MAX_CHARS);
        assert!(message.text().ends_with("\n[truncated]"));
    }

    #[test]
    fn long_formatted_builder_falls_back_to_truncated_plain_text() {
        let message = MessageBuilder::markdown_v2()
            .line("x_".repeat(SEND_MESSAGE_TEXT_MAX_CHARS))
            .build();

        assert_eq!(message.parse_mode(), ParseMode::Plain);
        assert_eq!(char_count(message.text()), SEND_MESSAGE_TEXT_MAX_CHARS);
        assert!(message.text().ends_with("\n[truncated]"));
    }

    #[test]
    fn default_value_budget_preserves_later_fields() {
        let message = MessageBuilder::plain()
            .error("error", "x".repeat(SEND_MESSAGE_TEXT_MAX_CHARS))
            .line("relay submissions disabled until process restart")
            .build();

        assert_eq!(message.parse_mode(), ParseMode::Plain);
        assert!(message.text().contains("\n[truncated]"));
        assert!(
            message
                .text()
                .contains("relay submissions disabled until process restart")
        );
        assert!(char_count(message.text()) < SEND_MESSAGE_TEXT_MAX_CHARS);
    }

    #[test]
    fn default_key_value_budget_preserves_later_key_values() {
        let message = MessageBuilder::plain()
            .kv("large", "x".repeat(SEND_MESSAGE_TEXT_MAX_CHARS))
            .kv("small", "high-signal")
            .build();

        assert!(message.text().contains("large: "));
        assert!(message.text().contains("\n[truncated]"));
        assert!(message.text().contains("small: high-signal"));
    }

    #[test]
    fn custom_value_budget_is_honored() {
        let message = MessageBuilder::plain()
            .kv_with_budget("payload", "abcdef", ValueBudget::Chars(16))
            .build();

        assert_eq!(message.text(), "payload: abcdef");

        let message = MessageBuilder::plain()
            .kv_with_budget("payload", "abcdefghijklmnopqrstu", ValueBudget::Chars(16))
            .build();

        assert_eq!(message.text(), "payload: abcd\n[truncated]");
    }

    #[test]
    fn unlimited_value_budget_uses_whole_message_truncation() {
        let message = MessageBuilder::plain()
            .error_with_budget(
                "error",
                "x".repeat(SEND_MESSAGE_TEXT_MAX_CHARS * 2),
                ValueBudget::Unlimited,
            )
            .line("relay submissions disabled until process restart")
            .build();

        assert_eq!(char_count(message.text()), SEND_MESSAGE_TEXT_MAX_CHARS);
        assert!(message.text().ends_with("\n[truncated]"));
        assert!(
            !message
                .text()
                .contains("relay submissions disabled until process restart")
        );
    }

    #[test]
    fn budgeted_markdown_code_block_keeps_complete_delimiters() {
        let message = MessageBuilder::markdown_v2()
            .error(
                "error",
                format!("`{}", "x".repeat(SEND_MESSAGE_TEXT_MAX_CHARS)),
            )
            .line("tail")
            .build();

        assert_eq!(message.parse_mode(), ParseMode::MarkdownV2);
        assert!(message.text().contains("```\n\\`"));
        assert!(message.text().contains("\n[truncated]\n```\ntail"));
    }

    #[test]
    fn try_build_untruncated_returns_error_for_long_message() {
        let error = MessageBuilder::plain()
            .line("x".repeat(SEND_MESSAGE_TEXT_MAX_CHARS + 1))
            .try_build_untruncated()
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "telegram message is 4097 chars, limit is 4096"
        );
    }

    #[test]
    fn callback_data_is_limited_to_sixty_four_bytes() {
        assert!(InlineKeyboardButton::callback("ok", "x".repeat(64)).is_ok());
        assert!(InlineKeyboardButton::callback("bad", "x".repeat(65)).is_err());
    }

    #[test]
    fn payload_contains_parse_mode_when_needed() {
        let message = MessageBuilder::markdown_v2().line("hello").build();
        let options = SendOptions::default();
        let request = SendMessageRequest::new("-100", &message, &options);
        let json = serde_json::to_value(request).unwrap();

        assert_eq!(json["chat_id"], "-100");
        assert_eq!(json["text"], "hello");
        assert_eq!(json["parse_mode"], "MarkdownV2");
    }
}
