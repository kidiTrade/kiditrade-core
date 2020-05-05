fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .format(true)
        .compile(&["../ibbridge/ibbridge.proto"], &["../ibbridge"])?;

    Ok(())
}
