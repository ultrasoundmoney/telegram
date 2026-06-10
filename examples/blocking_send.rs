#[cfg(feature = "blocking")]
fn main() -> Result<(), telegram::Error> {
    use telegram::{MessageBuilder, blocking::BlockingTelegramBot};

    let token = std::env::var("TELEGRAM_API_KEY").unwrap_or_else(|_| "TOKEN".to_string());
    let channel_id = std::env::var("TELEGRAM_CHANNEL_ID").unwrap_or_else(|_| "-100123".to_string());
    let bot = BlockingTelegramBot::new(token, channel_id);
    let message = MessageBuilder::plain().line("hello from telegram").build();

    bot.send(&message)?;

    Ok(())
}

#[cfg(not(feature = "blocking"))]
fn main() {
    eprintln!("enable the blocking feature to run this example");
}
