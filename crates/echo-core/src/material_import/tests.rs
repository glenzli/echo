use super::*;

#[test]
fn owned_material_survives_external_deletion_and_reimport_preserves_memory_membership() {
    let root = std::env::temp_dir().join(format!("echo-material-{}", echo_domain::AssetId::new()));
    fs::create_dir_all(&root).unwrap();
    let path = root.join("field.wav");
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(2036u32.to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1000u32.to_le_bytes());
    bytes.extend(2000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(2000u32.to_le_bytes());
    bytes.extend(vec![0; 2000]);
    fs::write(&path, bytes).unwrap();
    let catalog = echo_catalog::open_catalog(&root.join("library/catalog.sqlite")).unwrap();
    let request = MaterialImportPayload {
        path: path.clone(),
        assembly_id: String::new(),
        collect_globally: true,
        category: "ambience".into(),
    };
    import_material(&catalog, &request).unwrap();
    let asset = catalog
        .with_transaction(echo_catalog::list_assets)
        .unwrap()
        .remove(0);
    assert_ne!(asset.original.path, path);
    catalog
        .with_transaction(|tx| {
            echo_catalog::set_sound_membership(tx, &asset.id.to_string(), true, true, "ambience")
        })
        .unwrap();
    import_material(&catalog, &request).unwrap();
    fs::remove_file(path).unwrap();
    assert!(asset.original.path.is_file());
    let memberships = catalog
        .with_transaction(echo_catalog::sound_memberships)
        .unwrap();
    assert_eq!(memberships.len(), 1);
    assert!(memberships[&asset.id.to_string()].in_memory);
    assert!(memberships[&asset.id.to_string()].in_materials);
    drop(catalog);
    fs::remove_dir_all(root).unwrap();
}
