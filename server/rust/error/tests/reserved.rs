use tonic::Code;
use tonic_types::StatusExt;
use voltech_grpc_error::{Internal, failed_precondition, internal, reserved};

/// 予約エラーは domain voltech で、サーバーが返すものだけが一覧に入る。
#[test]
fn reserved_errors_use_reserved_domain() {
    assert_eq!(reserved::DOMAIN, "voltech");
    assert!(reserved::ALL.contains(&reserved::MAINTENANCE));
    assert!(reserved::ALL.iter().all(|code| code.domain() == "voltech"));
    // NETWORK_ERROR などクライアントだけが使うものは、サーバーの定数にしない
    assert!(
        reserved::ALL
            .iter()
            .all(|code| code.reason() != "NETWORK_ERROR")
    );
}

/// エラーコードの domain と reason が、そのまま ErrorInfo に入る。
#[test]
fn error_code_goes_into_error_info() {
    let status = failed_precondition(reserved::DATE_CHANGED, "game date changed");
    assert_eq!(status.code(), Code::FailedPrecondition);
    let info = status.get_details_error_info().unwrap();
    assert_eq!(info.domain, "voltech");
    assert_eq!(info.reason, "DATE_CHANGED");

    let info = internal(Internal::msg("boom"))
        .get_details_error_info()
        .unwrap();
    assert_eq!(info.reason, reserved::INTERNAL_ERROR.reason());
}
