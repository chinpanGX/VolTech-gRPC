pub mod sandbox {
    pub mod v1 {
        tonic::include_proto!("sandbox.v1");
    }
}

/// sandbox の .proto とその依存の FileDescriptorSet。冪等性の表と gRPC リフレクションに使う。
pub const FILE_DESCRIPTOR_SET: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/sandbox_descriptor.bin"));
