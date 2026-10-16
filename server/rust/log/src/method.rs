/// 呼ばれた RPC の名前。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpcMethod {
    service: String,
    method: String,
}

impl RpcMethod {
    /// gRPC の HTTP のパス(例: /app.v1.GachaService/Draw)から作る。形が違えば `None` を返す。
    pub fn from_path(path: &str) -> Option<Self> {
        let (service, method) = path.strip_prefix('/')?.split_once('/')?;
        if service.is_empty() || method.is_empty() {
            return None;
        }
        Some(Self {
            service: service.to_owned(),
            method: method.to_owned(),
        })
    }

    /// パッケージ付きのサービス名(例: app.v1.GachaService)。
    pub fn service(&self) -> &str {
        &self.service
    }

    /// メソッド名(例: Draw)。
    pub fn method(&self) -> &str {
        &self.method
    }

    /// サービス名とメソッド名をつないだ名前(例: app.v1.GachaService/Draw)。
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.service, self.method)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// gRPC のパスからサービス名とメソッド名を取り出し、形が違うパスは取り出さない。
    #[test]
    fn from_path_splits_service_and_method() {
        let method = RpcMethod::from_path("/app.v1.GachaService/Draw").unwrap();
        assert_eq!(method.service(), "app.v1.GachaService");
        assert_eq!(method.method(), "Draw");
        assert_eq!(method.full_name(), "app.v1.GachaService/Draw");

        for path in [
            "",
            "/",
            "app.v1.GachaService/Draw",
            "/app.v1.GachaService",
            "/app.v1.GachaService/",
        ] {
            assert_eq!(RpcMethod::from_path(path), None, "{path}");
        }
    }
}
