use std::collections::HashMap;

use prost::Message;
use prost_types::FileDescriptorSet;
use prost_types::method_options::IdempotencyLevel;

/// RPC の冪等性の扱い。`.proto` の `idempotency_level` から決まる。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Idempotency {
    /// `idempotency_level` を書いていない。RequestID で二重実行を防ぐ。
    Target,
    /// `NO_SIDE_EFFECTS`。読み取り専用のトランザクションで実行する。
    NoSideEffects,
    /// `IDEMPOTENT`。何回実行しても結果が同じなので、レスポンスを保存しない。
    Idempotent,
}

/// RPC ごとの冪等性の扱いの表。起動時に `.proto` の FileDescriptorSet から作る。
#[derive(Clone, Debug, Default)]
pub struct IdempotencyLevels {
    // キーは "パッケージ.サービス/メソッド"(例: app.v1.GachaService/Draw)
    by_method: HashMap<String, Idempotency>,
}

impl IdempotencyLevels {
    /// FileDescriptorSet(`include_bytes!` した `.bin`)を読んで作る。
    pub fn from_file_descriptor_set(bytes: &[u8]) -> Result<Self, prost::DecodeError> {
        let set = FileDescriptorSet::decode(bytes)?;
        let mut levels = Self::default();
        for file in &set.file {
            let package = file.package();
            for service in &file.service {
                for method in &service.method {
                    let level = method
                        .options
                        .as_ref()
                        .map(|options| options.idempotency_level())
                        .unwrap_or(IdempotencyLevel::IdempotencyUnknown);
                    let idempotency = match level {
                        IdempotencyLevel::NoSideEffects => Idempotency::NoSideEffects,
                        IdempotencyLevel::Idempotent => Idempotency::Idempotent,
                        IdempotencyLevel::IdempotencyUnknown => Idempotency::Target,
                    };
                    levels.by_method.insert(
                        format!("{package}.{}/{}", service.name(), method.name()),
                        idempotency,
                    );
                }
            }
        }
        Ok(levels)
    }

    /// 別の FileDescriptorSet から作った表を足す(アプリと基盤の `.proto` をまとめるとき)。
    pub fn merge(mut self, other: Self) -> Self {
        self.by_method.extend(other.by_method);
        self
    }

    /// RPC の扱いを返す。表に無い RPC は、二重実行を防ぐ側に倒して `Target` にする。
    pub fn get(&self, service: &str, method: &str) -> Idempotency {
        self.by_method
            .get(&format!("{service}/{method}"))
            .copied()
            .unwrap_or(Idempotency::Target)
    }
}

#[cfg(test)]
mod tests {
    use prost_types::{
        FileDescriptorProto, MethodDescriptorProto, MethodOptions, ServiceDescriptorProto,
    };

    use super::*;

    fn method(name: &str, level: Option<IdempotencyLevel>) -> MethodDescriptorProto {
        MethodDescriptorProto {
            name: Some(name.to_owned()),
            options: level.map(|level| MethodOptions {
                idempotency_level: Some(level as i32),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// `idempotency_level` の値ごとに扱いが決まり、書いていない RPC と表に無い RPC は Target になる。
    #[test]
    fn levels_follow_idempotency_level_option() {
        let set = FileDescriptorSet {
            file: vec![FileDescriptorProto {
                package: Some("app.v1".to_owned()),
                service: vec![ServiceDescriptorProto {
                    name: Some("SampleService".to_owned()),
                    method: vec![
                        method("Unwritten", None),
                        method("Read", Some(IdempotencyLevel::NoSideEffects)),
                        method("SignIn", Some(IdempotencyLevel::Idempotent)),
                    ],
                    ..Default::default()
                }],
                ..Default::default()
            }],
        };

        let levels = IdempotencyLevels::from_file_descriptor_set(&set.encode_to_vec()).unwrap();

        assert_eq!(
            levels.get("app.v1.SampleService", "Unwritten"),
            Idempotency::Target
        );
        assert_eq!(
            levels.get("app.v1.SampleService", "Read"),
            Idempotency::NoSideEffects
        );
        assert_eq!(
            levels.get("app.v1.SampleService", "SignIn"),
            Idempotency::Idempotent
        );
        assert_eq!(
            levels.get("app.v1.SampleService", "Missing"),
            Idempotency::Target
        );
    }
}
