# telegram

Small Telegram Bot API `sendMessage` client and safe message builders.

This crate focuses on operational messages:

- simple one-line messages
- line-oriented alerts
- key/value blocks
- key/value alerts with one large error/code field

It owns the final Telegram message before sending, so it can apply the
`sendMessage.text` limit to the full message instead of pretending fragment-level
sanitation can guarantee delivery.

Key/value and error values use a conservative per-value budget by default. This
keeps one huge value from hiding later fields. If a caller has already
normalized a value, it can opt that specific field out with
`ValueBudget::Unlimited` and rely on whole-message truncation instead.

## Features

```toml
default = ["async"]
async = ["reqwest", "tokio"]
blocking = ["ureq"]
```

The default async transport uses `reqwest`. The optional blocking transport uses
`ureq`.

## Logging

The send clients emit structured [`tracing`](https://docs.rs/tracing/) spans and
events. Callers decide whether those events are recorded by installing their own
subscriber.

The crate logs send attempts, successful message ids, API and HTTP failures, and
formatted-message fallback retries. Logs intentionally avoid tokens, request
URLs, chat ids, and message text.

## Version Tracking

The crate exposes its package version as `telegram::VERSION`:

```rust
tracing::info!(telegram.version = telegram::VERSION, "loaded telegram crate");
```

For internal services that depend on this repository directly, prefer pinning a
release tag:

```toml
telegram = { git = "https://github.com/ultrasoundmoney/telegram", tag = "v0.2.0" }
```

## Examples

Plain alert:

```rust
use telegram::{MessageBuilder, ParseMode};

let message = MessageBuilder::new(ParseMode::Plain)
    .line("cielago alert: relay submissions disabled")
    .kv("label", "rbx-prod-mainnet")
    .kv("aggregation_slot", 12345)
    .kv("reason", "bundle tracer failed to flush records to postgres")
    .error("error", "database unavailable")
    .line("relay submissions disabled until process restart")
    .build();
```

MarkdownV2 alert with a large error:

```rust
use telegram::{MessageBuilder, ParseMode};

let message = MessageBuilder::new(ParseMode::MarkdownV2)
    .bold_line("builder demoted")
    .kv_code("slot", 12345)
    .error("error", "simulation failed:\ninvalid `root`")
    .build();
```

Caller-controlled value budget:

```rust
use telegram::{MessageBuilder, ParseMode, ValueBudget};

let compact_error = "caller already summarized this error";
let message = MessageBuilder::new(ParseMode::Plain)
    .kv("label", "rbx-prod-mainnet")
    .error_with_budget("error", compact_error, ValueBudget::Unlimited)
    .build();
```

Async send:

```rust,no_run
use telegram::{MessageBuilder, ParseMode, TelegramBot};

#[tokio::main]
async fn main() -> Result<(), telegram::Error> {
    let bot = TelegramBot::new("TOKEN", "-100123");
    let message = MessageBuilder::new(ParseMode::Plain)
        .line("hello from telegram")
        .build();

    bot.send(&message).await?;
    Ok(())
}
```

Blocking send:

```rust,no_run
use telegram::{blocking::BlockingTelegramBot, MessageBuilder, ParseMode};

fn main() -> Result<(), telegram::Error> {
    let bot = BlockingTelegramBot::new("TOKEN", "-100123");
    let message = MessageBuilder::new(ParseMode::Plain)
        .line("hello from telegram")
        .build();

    bot.send(&message)?;
    Ok(())
}
```
