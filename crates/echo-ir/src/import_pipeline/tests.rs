use std::{fs, path::PathBuf};

use super::*;
use crate::{IrRightsDeclaration, IrStoreErrorKind};

fn root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "echo-ir-pipeline-{name}-{}-{}",
        std::process::id(),
        Uuid::now_v7()
    ))
}

fn append_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn pcm16_mono_wav(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
    let data_bytes = u32::try_from(samples.len() * 2).expect("bounded fixture");
    let mut wav = Vec::with_capacity(44 + samples.len() * 2);
    wav.extend_from_slice(b"RIFF");
    append_u32(&mut wav, 36 + data_bytes);
    wav.extend_from_slice(b"WAVEfmt ");
    append_u32(&mut wav, 16);
    append_u16(&mut wav, 1);
    append_u16(&mut wav, 1);
    append_u32(&mut wav, sample_rate);
    append_u32(&mut wav, sample_rate * 2);
    append_u16(&mut wav, 2);
    append_u16(&mut wav, 16);
    wav.extend_from_slice(b"data");
    append_u32(&mut wav, data_bytes);
    for sample in samples {
        append_u16(&mut wav, u16::from_le_bytes(sample.to_le_bytes()));
    }
    wav
}

fn user_owned(name: &str) -> IrImportProvenance {
    IrImportProvenance {
        display_name: name.to_owned(),
        creator: Some("Fixture recorder".to_owned()),
        source_url: None,
        attribution: None,
        rights: IrRightsDeclaration::UserOwnedNoRedistribution,
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("four bytes"))
}

#[test]
fn local_wav_closes_source_preparation_and_provenance_lifecycles() {
    let root = root("closure");
    fs::create_dir_all(&root).expect("create root");
    let source_path = root.join("small-room.wav");
    let mut samples = vec![0_i16; 2400];
    samples[0] = 28000;
    samples[333] = -12000;
    fs::write(&source_path, pcm16_mono_wav(24000, &samples)).expect("write fixture");
    let pipeline =
        IrImportPipeline::open(&root.join("sources"), &root.join("cache")).expect("open pipeline");

    let imported = pipeline
        .import_local_wav(&source_path, user_owned("Small room"))
        .expect("import and prepare");
    assert_eq!(imported.source_outcome, ImportOutcome::Stored);
    assert_eq!(imported.preparation.outcome, PreparationOutcome::Stored);
    assert_eq!(
        imported.preparation.source_hash,
        imported.record.source_hash
    );
    assert_eq!(imported.preparation.source_sample_rate, 24000);
    assert_eq!(imported.preparation.channel_count, 1);
    assert_eq!(imported.preparation.source_frame_count, 2400);
    assert_eq!(imported.preparation.prepared_frame_count, 4800);
    let prepared = fs::read(&imported.preparation.cache_path).expect("read prepared cache");
    assert_eq!(&prepared[..8], b"ECHOIR01");
    assert_eq!(
        read_u32(&prepared, 12),
        imported.preparation.preparation_version
    );
    assert_eq!(read_u32(&prepared, 16), 48000);
    assert_eq!(
        ContentHash::from(blake3::hash(&prepared)),
        imported.preparation.prepared_hash
    );
    assert_eq!(
        u64::try_from(prepared.len()).expect("prepared size"),
        imported.preparation.size_bytes
    );
    assert!(
        pipeline
            .source_store()
            .provenance_path(imported.record.import_id)
            .exists()
    );

    fs::remove_file(&source_path).expect("remove original import path");
    pipeline
        .source_store()
        .verify_source(
            imported.record.source_hash,
            imported.record.source_size_bytes,
        )
        .expect("durable source remains");
    fs::remove_file(&imported.preparation.cache_path).expect("evict prepared cache");
    let rebuilt = pipeline
        .prepare_owned_source(&StoredIrSource {
            source_hash: imported.record.source_hash,
            source_size_bytes: imported.record.source_size_bytes,
            outcome: ImportOutcome::AlreadyPresent,
        })
        .expect("rebuild from owned source");
    assert_eq!(rebuilt.outcome, PreparationOutcome::Stored);
    assert_eq!(rebuilt.prepared_hash, imported.preparation.prepared_hash);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn duplicate_bytes_deduplicate_both_lifecycles_but_append_provenance() {
    let root = root("dedup");
    fs::create_dir_all(&root).expect("create root");
    let bytes = pcm16_mono_wav(48000, &[24000, 0, 0, -12000]);
    let first_path = root.join("first.wav");
    let second_path = root.join("second.wav");
    fs::write(&first_path, &bytes).expect("write first");
    fs::write(&second_path, &bytes).expect("write second");
    let pipeline =
        IrImportPipeline::open(&root.join("sources"), &root.join("cache")).expect("open pipeline");

    let first = pipeline
        .import_local_wav(&first_path, user_owned("First title"))
        .expect("first import");
    let second = pipeline
        .import_local_wav(&second_path, user_owned("Corrected title"))
        .expect("second import");
    assert_eq!(first.source_outcome, ImportOutcome::Stored);
    assert_eq!(second.source_outcome, ImportOutcome::AlreadyPresent);
    assert_eq!(first.preparation.outcome, PreparationOutcome::Stored);
    assert_eq!(
        second.preparation.outcome,
        PreparationOutcome::AlreadyPresent
    );
    assert_eq!(first.record.source_hash, second.record.source_hash);
    assert_eq!(
        first.preparation.prepared_hash,
        second.preparation.prepared_hash
    );
    assert_ne!(first.record.import_id, second.record.import_id);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn invalid_rights_or_wav_never_publish_provenance() {
    let root = root("fail-closed");
    fs::create_dir_all(&root).expect("create root");
    let valid_path = root.join("valid.wav");
    fs::write(&valid_path, pcm16_mono_wav(48000, &[12000])).expect("write valid");
    let pipeline =
        IrImportPipeline::open(&root.join("sources"), &root.join("cache")).expect("open pipeline");
    let invalid_rights = IrImportProvenance {
        display_name: "Invalid".to_owned(),
        creator: None,
        source_url: None,
        attribution: None,
        rights: IrRightsDeclaration::Spdx {
            expression: " ".to_owned(),
            license_url: None,
        },
    };
    let rights_error = pipeline
        .import_local_wav(&valid_path, invalid_rights)
        .expect_err("reject rights before source publication");
    assert_eq!(rights_error.kind, IrStoreErrorKind::InvalidInput);
    assert_eq!(
        fs::read_dir(root.join("sources/sources/b3"))
            .expect("read source objects")
            .count(),
        0
    );

    let silent_path = root.join("silent.wav");
    fs::write(&silent_path, pcm16_mono_wav(48000, &[0, 0, 0, 0])).expect("write silent");
    let preparation_error = pipeline
        .import_local_wav(&silent_path, user_owned("Silent"))
        .expect_err("reject silent IR");
    assert_eq!(
        preparation_error.kind,
        IrStoreErrorKind::PreparationRejected
    );
    assert_eq!(
        fs::read_dir(root.join("sources/provenance/events"))
            .expect("read provenance events")
            .count(),
        0
    );
    let _ = fs::remove_dir_all(root);
}
