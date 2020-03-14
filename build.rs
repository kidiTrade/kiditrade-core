fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .format(true)
        .compile(&["../ib_loader/ibbridge.proto"], &["../ib_loader"])?;

    Ok(())
}
