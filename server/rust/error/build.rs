fn main() -> Result<(), Box<dyn std::error::Error>> {
    voltech_grpc_error_build::configure()
        .reserved()
        .compile("errors.toml")
}
