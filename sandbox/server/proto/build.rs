use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use prost::Message;

/// sandbox の .proto の置き場所。
const SANDBOX_PROTO_ROOT: &str = "../../proto";
/// sandbox の業務エラーの定義ファイル。
const ERRORS: &str = "../../errors.toml";

fn main() -> Result<(), Box<dyn Error>> {
    // protoc を入れなくてもビルドできるように、Rust 製のコンパイラ(protox)で読み込む
    let fds = protox::compile(["sandbox/v1/sandbox.proto"], [SANDBOX_PROTO_ROOT])?;

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    fs::write(out_dir.join("sandbox_descriptor.bin"), fds.encode_to_vec())?;

    // クライアントは server の結合テストで使う
    tonic_prost_build::configure().compile_fds(fds)?;

    voltech_grpc_error_build::compile(ERRORS)?;

    println!("cargo:rerun-if-changed={SANDBOX_PROTO_ROOT}");
    Ok(())
}
