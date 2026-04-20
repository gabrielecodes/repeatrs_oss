fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .field_attribute(
            "repeatrs.v1.QueueItem.created_at",
            "#[serde(with = \"crate::timestamp_format\")]",
        )
        .field_attribute(
            "repeatrs.v1.JobItem.next_occurrence_at",
            "#[serde(with = \"crate::timestamp_format\")]",
        )
        .compile_protos(&["proto/repeatrs.proto"], &["proto"])?;
    Ok(())
}
