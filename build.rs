fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .format(true)
        .compile(&["../ib_loader/ib_loader.proto"], &["../ib_loader"])?;

    Ok(())
}
