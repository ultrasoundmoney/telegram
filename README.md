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

## Features

```toml
default = ["async"]
async = ["reqwest", "tokio"]
blocking = ["ureq"]
```

The default async transport uses `reqwest`. The optional blocking transport uses
`ureq`.

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
