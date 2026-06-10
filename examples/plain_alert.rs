use telegram::{MessageBuilder, ParseMode};

fn main() {
    let message = MessageBuilder::new(ParseMode::Plain)
        .line("cielago alert: relay submissions disabled")
        .kv("label", "rbx-prod-mainnet")
        .kv("aggregation_slot", 12345)
        .kv(
            "reason",
            "bundle tracer failed to flush records to postgres",
        )
        .error("error", "database unavailable")
        .line("relay submissions disabled until process restart")
        .build();

    println!("{}", message.text());
}
