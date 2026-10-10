use proto::sandbox::v1::ErrorCode;

/// 基盤が返す予約コードの名前が、すべて sandbox の error_code.proto にある。
#[test]
fn package_reserved_codes_exist_in_error_code_proto() {
    for name in voltech_grpc_error::code::ALL {
        assert!(
            ErrorCode::from_str_name(name).is_some(),
            "{name} is missing in error_code.proto"
        );
    }
}
