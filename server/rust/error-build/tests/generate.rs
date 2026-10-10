use voltech_grpc_error_build::configure;

const APP: &str = r#"
domain = "my-app"

[[errors]]
reason = "INSUFFICIENT_GEMS"
action = "stay"
description = "ジェムが足りない。"

[[errors]]
reason = "OFFLINE_ONLY"
action = "title"
client_only = true
description = "クライアントだけが使う。"
"#;

/// サーバーが返すエラーだけが、アプリの domain の定数と一覧になる。
#[test]
fn generates_constants_for_server_errors() {
    let code = configure().generate(APP).unwrap();

    assert!(code.contains("pub const DOMAIN: &str = \"my-app\";"));
    assert!(code.contains(
        "pub const INSUFFICIENT_GEMS: ::voltech_grpc_error::ErrorCode = ::voltech_grpc_error::ErrorCode::new(DOMAIN, \"INSUFFICIENT_GEMS\");"
    ));
    assert!(code.contains("/// ジェムが足りない。"));
    assert!(
        code.contains(
            "pub const ALL: &[::voltech_grpc_error::ErrorCode] = &[INSUFFICIENT_GEMS, ];"
        )
    );
    assert!(!code.contains("OFFLINE_ONLY"));
}

/// アプリは基盤の予約エラーの domain を使えない。
#[test]
fn app_cannot_use_reserved_domain() {
    let source = APP.replace("my-app", "voltech");
    let err = configure().generate(&source).unwrap_err();
    assert!(err.to_string().contains("voltech"), "{err}");
}

/// 基盤の予約エラーは、domain voltech でなければならない。
#[test]
fn reserved_requires_reserved_domain() {
    assert!(configure().reserved().generate(APP).is_err());
}

/// 規則に合わない定義は弾く。
#[test]
fn rejects_invalid_definitions() {
    let cases = [
        (
            "reason が小文字",
            APP.replace("INSUFFICIENT_GEMS", "insufficient_gems"),
        ),
        (
            "reason が _ で終わる",
            APP.replace("INSUFFICIENT_GEMS", "INSUFFICIENT_"),
        ),
        (
            "reason が長すぎる",
            APP.replace("INSUFFICIENT_GEMS", &"A".repeat(64)),
        ),
        (
            "reason が重複",
            APP.replace("OFFLINE_ONLY", "INSUFFICIENT_GEMS"),
        ),
        (
            "生成する定数とぶつかる",
            APP.replace("\"OFFLINE_ONLY\"", "\"ALL\""),
        ),
        ("domain が大文字", APP.replace("my-app", "MyApp")),
        ("知らない action", APP.replace("\"stay\"", "\"dialog\"")),
        (
            "知らない項目",
            APP.replace("client_only = true", "client = true"),
        ),
        ("description が空", APP.replace("ジェムが足りない。", "")),
    ];
    for (name, source) in cases {
        assert!(configure().generate(&source).is_err(), "{name}");
    }
}
