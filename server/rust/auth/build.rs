use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use prost::Message;

const PROTO_ROOT: &str = "proto";

fn main() -> Result<(), Box<dyn Error>> {
    // protoc を入れなくてもビルドできるように、Rust 製のコンパイラ(protox)で読み込む
    let fds = protox::compile(["voltech/v1/device.proto"], [PROTO_ROOT])?;

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    fs::write(out_dir.join("auth_descriptor.bin"), fds.encode_to_vec())?;

    tonic_prost_build::configure().compile_fds(fds)?;

    println!("cargo:rerun-if-changed={PROTO_ROOT}");
    Ok(())
}
