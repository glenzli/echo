use super::*;
#[test]
fn expanded_formats_keep_referenced_ids_and_old_exports() {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(crate::schema::SCHEMA_SQL).unwrap();
    connection
        .execute_batch(crate::schema::SOUND_ASSEMBLY_MIGRATION_SQL)
        .unwrap();
    connection.execute_batch("CREATE TABLE kept_export(id INTEGER REFERENCES render_exports(id)); INSERT INTO assets (id,content_hash,path,path_status,size_bytes,codec,duration_millis,recorded_at_millis,imported_at_millis) VALUES('asset','hash','/tmp/voice.wav','present',44,'pcm',1000,0,1); INSERT INTO render_exports VALUES(7,'asset',NULL,'/tmp/out.wav','wav_pcm24',48000,2,24,48000,'render-hash',288044,-12,-1,1); INSERT INTO kept_export VALUES(7);").unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    migrate(&connection).unwrap();
    assert!(
        connection
            .pragma_query_value(None, "foreign_keys", |row| row.get::<_, bool>(0))
            .unwrap()
    );
    let count: i64 = connection
        .query_row(
            "SELECT count(*) FROM kept_export JOIN render_exports USING(id)",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    connection.execute("INSERT INTO render_exports (asset_id,output_path,format,sample_rate,channel_count,bit_depth,frame_count,content_hash,size_bytes,integrated_lufs,true_peak_dbtp,created_at_millis) VALUES('asset','/tmp/out.mp3','mp3',44100,1,0,44100,'compressed-hash',100,-12,-1,2)",[]).unwrap();
    migrate(&connection).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM render_exports", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
}
