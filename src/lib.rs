//! Minimal Telegram Bot API `sendMessage` client and safe message builders.
//!
//! The crate focuses on operational messages: simple lines, key/value blocks,
//! and alerts with a large error/code field. It owns the complete Telegram
//! message before sending, so it can apply Telegram's message length limit to
//! the whole message.
//!
//! Feature flags:
//!
//! - `async` is enabled by default and provides [`TelegramBot`] via `reqwest`.
//! - `blocking` provides [`blocking::BlockingTelegramBot`] via `ureq`.
//!
//! ```
//! use telegram::{MessageBuilder, ParseMode};
//!
//! let message = MessageBuilder::new(ParseMode::Plain)
//!     .line("cielago alert: relay submissions disabled")
//!     .kv("label", "rbx-prod-mainnet")
//!     .error("error", "database unavailable")
//!     .build();
//!
//! assert_eq!(message.parse_mode_parameter(), None);
//! ```

mod error;
mod message;

#[cfg(feature = "async")]
mod async_bot;

#[cfg(feature = "blocking")]
pub mod blocking {
    pub use crate::blocking_bot::BlockingTelegramBot;
}

#[cfg(feature = "blocking")]
mod blocking_bot;

#[cfg(feature = "async")]
pub use async_bot::TelegramBot;
pub use error::Error;
pub use message::{
    CALLBACK_DATA_MAX_BYTES, DEFAULT_TIMEOUT, InlineKeyboardButton, InlineKeyboardMarkup,
    MessageBuilder, ParseFailureFallback, ParseMode, SendOptions, SentMessage, TelegramMessage,
};

pub use telegram_sanitize::SEND_MESSAGE_TEXT_MAX_CHARS;

pub(crate) use message::{ApiResponse, SendMessageRequest, fallback_plain_message};
