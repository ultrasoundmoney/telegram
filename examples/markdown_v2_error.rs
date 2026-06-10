use telegram::MessageBuilder;

fn main() {
    let message = MessageBuilder::markdown_v2()
        .bold_line("builder demoted")
        .kv_code("slot", 12345)
        .kv_code("builder_id", "beaver_build[prod]")
        .error(
            "error",
            "simulation failed:\ninvalid `root` from \\ upstream",
        )
        .build();

    println!("{}", message.text());
}
