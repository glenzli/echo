//! Explicit IR plane intent crosses the desktop facade without channel-count inference.

use crate::session::open_session;

fn append_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_i24(bytes: &mut Vec<u8>, value: i32) {
    let bits = value.cast_unsigned();
    bytes.push((bits & 0xff) as u8);
    bytes.push(((bits >> 8) & 0xff) as u8);
    bytes.push(((bits >> 16) & 0xff) as u8);
}

fn true_stereo_pcm24_wav() -> Vec<u8> {
    const SAMPLE_RATE: u32 = 48_000;
    const CHANNELS: u16 = 4;
    const FRAMES: u32 = 4;
    const DATA_BYTES: u32 = FRAMES * 4 * 3;
    let mut wav = Vec::with_capacity((68 + DATA_BYTES) as usize);
    wav.extend_from_slice(b"RIFF");
    append_u32(&mut wav, 60 + DATA_BYTES);
    wav.extend_from_slice(b"WAVEfmt ");
    append_u32(&mut wav, 40);
    append_u16(&mut wav, 0xfffe);
    append_u16(&mut wav, CHANNELS);
    append_u32(&mut wav, SAMPLE_RATE);
    append_u32(&mut wav, SAMPLE_RATE * u32::from(CHANNELS) * 3);
    append_u16(&mut wav, CHANNELS * 3);
    append_u16(&mut wav, 24);
    append_u16(&mut wav, 22);
    append_u16(&mut wav, 24);
    append_u32(&mut wav, 0x33);
    append_u32(&mut wav, 1);
    append_u16(&mut wav, 0);
    append_u16(&mut wav, 0x0010);
    wav.extend_from_slice(&[0x80, 0, 0, 0xaa, 0, 0x38, 0x9b, 0x71]);
    wav.extend_from_slice(b"data");
    append_u32(&mut wav, DATA_BYTES);
    for frame in 0..FRAMES {
        append_i24(&mut wav, if frame == 0 { 0x0040_0000 } else { 0 });
        append_i24(&mut wav, if frame == 1 { 0x0030_0000 } else { 0 });
        append_i24(&mut wav, if frame == 2 { -0x0040_0000 } else { 0 });
        append_i24(&mut wav, if frame == 3 { -0x0020_0000 } else { 0 });
    }
    wav
}

#[test]
fn true_stereo_import_requires_explicit_layout_and_rebuilds_from_catalog_evidence() {
    let root = std::env::temp_dir().join(format!(
        "echo-desktop-true-stereo-ir-{}-{}",
        std::process::id(),
        uuid::Uuid::now_v7()
    ));
    let session = open_session(
        root.join("catalog.sqlite").to_str().expect("utf8"),
        root.join("cache").to_str().expect("utf8"),
    )
    .expect("session opens");
    let source = root.join("true-stereo.wav");
    std::fs::write(&source, true_stereo_pcm24_wav()).expect("fixture writes");
    assert!(
        session
            .import_impulse_response(
                source.to_str().expect("utf8"),
                "Must not infer",
                "",
                "",
                "",
                "user_owned_no_redistribution",
                "",
                "",
            )
            .is_err()
    );
    assert!(
        session
            .import_impulse_response_with_layout(
                source.to_str().expect("utf8"),
                "four_channel",
                "Invalid intent",
                "",
                "",
                "",
                "user_owned_no_redistribution",
                "",
                "",
            )
            .is_err()
    );
    assert!(
        session
            .catalog()
            .with_transaction(echo_catalog::list_impulse_responses)
            .expect("catalog reads")
            .is_empty()
    );

    let imported = session
        .import_impulse_response_with_layout(
            source.to_str().expect("utf8"),
            "true_stereo_ll_lr_rl_rr",
            "True stereo room",
            "",
            "",
            "",
            "user_owned_no_redistribution",
            "",
            "",
        )
        .expect("explicit true stereo imports");
    assert_eq!(imported.layout_kind, "true_stereo_ll_lr_rl_rr");
    assert_eq!(imported.preparation_version, 2);
    assert_eq!(imported.channel_count, 4);
    std::fs::remove_file(&imported.prepared_path).expect("prepared cache evicts");
    let rebuilt = session
        .impulse_responses()
        .expect("Catalog intent rebuilds");
    assert_eq!(rebuilt.len(), 1);
    assert_eq!(rebuilt[0].prepared_hash, imported.prepared_hash);
    assert_eq!(rebuilt[0].layout_kind, imported.layout_kind);
    assert_eq!(rebuilt[0].preparation_version, 2);
    assert_eq!(rebuilt[0].channel_count, 4);
    let _ = std::fs::remove_dir_all(root);
}
