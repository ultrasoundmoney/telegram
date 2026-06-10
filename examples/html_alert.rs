use telegram::MessageBuilder;

fn main() {
    let message = MessageBuilder::html()
        .bold_line("builder demoted")
        .kv_code("builder_id", "beaverbuild <prod>")
        .error(
            "error",
            "simulation failed: invalid <root> & upstream rejected payload",
        )
        .build();

    println!("{}", message.text());
}
