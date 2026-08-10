use super::*;

#[test]
fn cjk_queries_and_literal_quotes_remain_valid_phrases() {
    assert_eq!(segment_cjk("小火车 rain"), "小 火 车 rain");
    assert_eq!(fts_phrase("rain \"station\""), "\"rain \"\"station\"\"\"");

    let root = std::env::temp_dir().join(format!(
        "echo-search-phrase-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t")
    ));
    let catalog = crate::open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    catalog
        .with_transaction(|transaction| index_transcript(transaction, "fixture", "雨中的小火车"))
        .expect("transcript indexes");
    let hits = catalog
        .with_transaction(|transaction| search_transcripts(transaction, "小火车", 10))
        .expect("CJK phrase searches");
    assert_eq!(hits.len(), 1);
    catalog
        .with_transaction(|transaction| search_transcripts(transaction, "小\"火车", 10))
        .expect("embedded quote cannot break FTS syntax");

    drop(catalog);
    let _ = std::fs::remove_dir_all(root);
}
