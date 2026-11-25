fn labels() -> (Option<&'static str>, Option<String>) {
    let baked = option_env!("PREVIEW_LABEL");
    let runtime = std::env::var("PREVIEW_LABEL").ok();
    (baked, runtime)
}
