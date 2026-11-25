fn title() -> &'static str {
    let caption = {
        let _ = std::env::var("MODE");
        "Catalog"
    };
    caption
}
