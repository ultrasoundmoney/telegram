use std::{fmt, time::Duration};

use serde::{Deserialize, Serialize};
use telegram_sanitize::{SEND_MESSAGE_TEXT_MAX_CHARS, html, markdown_v2, plain_text};

use crate::Error;

pub const CALLBACK_DATA_MAX_BYTES: usize = 64;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(6);

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

    pub fn kv(mut self, key: impl fmt::Display, value: impl fmt::Display) -> Self {
        self.blocks.push(Block::KeyValue {
            key: key.to_string(),
            value: Value::Text(value.to_string()),
        });
        self
    }

    pub fn kv_code(mut self, key: impl fmt::Display, value: impl fmt::Display) -> Self {
        self.blocks.push(Block::KeyValue {
            key: key.to_string(),
            value: Value::InlineCode(value.to_string()),
        });
        self
    }

    pub fn error(mut self, key: impl fmt::Display, value: impl fmt::Display) -> Self {
        self.blocks.push(Block::MultilineValue {
            key: key.to_string(),
            value: value.to_string(),
        });
        self
    }

    pub fn code_block(self, key: impl fmt::Display, value: impl fmt::Display) -> Self {
        self.error(key, value)
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
    KeyValue { key: String, value: Value },
    MultilineValue { key: String, value: String },
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
            Self::MultilineValue { key, value } => match parse_mode {
                ParseMode::Plain => format!("{}:\n{}", plain_text(key), plain_text(value)),
                ParseMode::MarkdownV2 => {
                    format!(
                        "{}:\n{}",
                        markdown_v2::text(key),
                        markdown_v2::code_block(value)
                    )
                }
                ParseMode::Html => format!("{}:\n{}", html::text(key), html::pre(value)),
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
    Text(String),
    InlineCode(String),
}

impl Value {
    fn render(&self, parse_mode: ParseMode) -> String {
        match self {
            Self::Text(value) => render_text(value, parse_mode),
            Self::InlineCode(value) => match parse_mode {
                ParseMode::Plain => plain_text(value),
                ParseMode::MarkdownV2 => markdown_v2::inline_code(value),
                ParseMode::Html => html::code(value),
            },
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
    const MARKER: &str = "\n[truncated]";

    if char_count(input) <= SEND_MESSAGE_TEXT_MAX_CHARS {
        input.to_string()
    } else {
        let marker_chars = char_count(MARKER);
        let keep = SEND_MESSAGE_TEXT_MAX_CHARS.saturating_sub(marker_chars);
        input.chars().take(keep).chain(MARKER.chars()).collect()
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
