//! PR C measurement checkpoint; no allocator or real-corpus acceptance claim.
#[test]
fn fixture_and_generator_storage_accounting() {
    prism::build_pool::install(|| {
        let mut rows = Vec::new();
        super::cases::visit(|name, files| {
            let mut row = prism::data_flow::test_support::accounting::fixture(&files);
            row["case"] = name.into();
            rows.push(row);
        });
        assert_eq!(rows.len(), super::pins::parent_pins().len());
        for (name, mut row) in prism::data_flow::test_support::accounting::synthetic() {
            row["case"] = format!("synthetic-{name}").into();
            rows.push(row);
        }
        if let Ok(dir) = std::env::var("DFG_ACCOUNTING_DIR") {
            let dir = std::path::Path::new(&dir);
            std::fs::create_dir_all(dir).unwrap();
            let text = rows
                .iter()
                .map(|r| format!("{}\n", serde_json::to_string(r).unwrap()))
                .collect::<String>();
            std::fs::write(dir.join("fixture-accounting.jsonl"), text).unwrap();
        }
        eprintln!(
            "DFG_ACCOUNTING {} fixture/generator + 3 synthetic rows",
            rows.len() - 3
        );
    });
}
