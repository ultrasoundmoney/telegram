#[cfg(feature = "async")]
#[tokio::main]
async fn main() -> Result<(), telegram::Error> {
    use telegram::{MessageBuilder, TelegramBot};

    let token = std::env::var("TELEGRAM_API_KEY").unwrap_or_else(|_| "TOKEN".to_string());
    let channel_id = std::env::var("TELEGRAM_CHANNEL_ID").unwrap_or_else(|_| "-100123".to_string());
    let bot = TelegramBot::new(token, channel_id);
    let message = MessageBuilder::plain().line("hello from telegram").build();

    bot.send(&message).await?;

    Ok(())
}

#[cfg(not(feature = "async"))]
fn main() {
    eprintln!("enable the async feature to run this example");
}
