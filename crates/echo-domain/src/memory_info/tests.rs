use super::*;

#[test]
fn approximate_context_and_source_anchors_are_not_invented_timestamps() {
    let value = MemoryInfo {
        notes: "  第一次叫爸爸\r\n很小声  ".into(),
        place: "外婆家阳台".into(),
        time_description: "大约 2020 年夏天".into(),
        moments: vec![MemoryMoment {
            start_millis: 40,
            end_millis: Some(90),
            note: "这里".into(),
        }],
    };
    let clean = value.clone().normalized(Some(100)).unwrap();
    assert_eq!(clean.notes, "第一次叫爸爸\n很小声");
    assert_eq!(clean.time_description, value.time_description);
    assert!(value.clone().normalized(Some(80)).is_err());
    assert!(value.normalized(None).is_err());
    for invalid in ["bad\0place".to_owned(), "x".repeat(201)] {
        assert!(
            MemoryInfo {
                place: invalid,
                ..MemoryInfo::default()
            }
            .normalized(None)
            .is_err()
        );
    }
}
