use std::str::FromStr;

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};
use serde::Deserialize;

use crate::BoxError;

/// サーバーの状態の設定(`server_status.toml` の中身)。読み込むときに値を確かめてある。
#[derive(Clone, Debug)]
pub struct StatusConfig {
    pub maintenance: Maintenance,
    pub min_version: MinVersion,
    pub game_day: GameDay,
}

impl StatusConfig {
    /// TOML の文字列から読む。知らないキーや範囲外の値があればエラーにする(書き間違いに気づけるように)。
    pub fn from_toml(text: &str) -> Result<Self, BoxError> {
        let raw: RawConfig = toml::from_str(text)?;
        let end_at = raw
            .maintenance
            .end_at
            .map(|value| value.parse::<Timestamp>())
            .transpose()
            .map_err(|err| format!("maintenance.end_at: {err}"))?;
        if !(0..=23).contains(&raw.game_day.reset_hour) {
            return Err(format!(
                "game_day.reset_hour must be 0..=23, got {}",
                raw.game_day.reset_hour
            )
            .into());
        }
        let timezone = TimeZone::get(&raw.game_day.timezone)
            .map_err(|err| format!("game_day.timezone: {err}"))?;

        Ok(Self {
            maintenance: Maintenance {
                enabled: raw.maintenance.enabled,
                end_at,
            },
            min_version: raw.min_version,
            game_day: GameDay {
                reset_hour: raw.game_day.reset_hour,
                timezone,
            },
        })
    }
}

/// メンテナンスの設定。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Maintenance {
    pub enabled: bool,
    /// ダイアログに出す終了予定。時刻が来ても自動では終わらない。
    pub end_at: Option<Timestamp>,
}

/// プラットフォームごとの最低バージョン(リリースのたびに増やす整数)。これより小さいバージョンには `UPDATE_REQUIRED` を返す。
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MinVersion {
    pub ios: u32,
    pub android: u32,
    pub windows: u32,
}

impl MinVersion {
    /// `platform` の最低バージョン。エディタは強制アップデートを判定しないので `None`。
    pub fn get(&self, platform: Platform) -> Option<u32> {
        match platform {
            Platform::Ios => Some(self.ios),
            Platform::Android => Some(self.android),
            Platform::Windows => Some(self.windows),
            Platform::Editor => None,
        }
    }
}

/// アプリのプラットフォーム。リクエストヘッダー `x-app-platform` の値。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Ios,
    Android,
    Windows,
    /// Unity のエディタ。開発中に最低バージョンで止まらないように、強制アップデートを判定しない。
    Editor,
}

impl FromStr for Platform {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ios" => Ok(Self::Ios),
            "android" => Ok(Self::Android),
            "windows" => Ok(Self::Windows),
            "editor" => Ok(Self::Editor),
            _ => Err(()),
        }
    }
}

/// ゲーム内の日付の決め方。
#[derive(Clone, Debug)]
pub struct GameDay {
    reset_hour: i8,
    timezone: TimeZone,
}

impl GameDay {
    /// `now` のときのゲーム内の日付。`timezone` の時刻で `reset_hour` 時に日付が変わる。
    pub fn date_at(&self, now: Timestamp) -> Date {
        let local = now.to_zoned(self.timezone.clone()).datetime();
        // 壁時計の時刻から reset_hour 時間を引いた日付にする(夏時間の切り替えの日も、時刻の表示どおりに変わる)
        local
            .checked_sub(i64::from(self.reset_hour).hours())
            .unwrap_or(local)
            .date()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    maintenance: RawMaintenance,
    min_version: MinVersion,
    game_day: RawGameDay,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMaintenance {
    enabled: bool,
    end_at: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGameDay {
    reset_hour: i8,
    timezone: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[maintenance]
enabled = true
end_at = "2026-10-01T15:00:00+09:00"

[min_version]
ios = 120
android = 121
windows = 122

[game_day]
reset_hour = 4
timezone = "Asia/Tokyo"
"#;

    /// 設計書の形の TOML を読み、終了予定は時刻として、最低バージョンはプラットフォームごとに取り出せる(エディタは無い)。
    #[test]
    fn parses_sample() {
        let config = StatusConfig::from_toml(SAMPLE).unwrap();
        assert!(config.maintenance.enabled);
        assert_eq!(
            config.maintenance.end_at,
            Some("2026-10-01T06:00:00Z".parse().unwrap())
        );
        assert_eq!(config.min_version.get(Platform::Ios), Some(120));
        assert_eq!(config.min_version.get(Platform::Android), Some(121));
        assert_eq!(config.min_version.get(Platform::Windows), Some(122));
        assert_eq!(config.min_version.get(Platform::Editor), None);
    }

    /// 書き間違い(知らないキー、範囲外の時刻、知らないタイムゾーン、時刻でない終了予定)はエラーになる。
    #[test]
    fn rejects_invalid_values() {
        for (from, to) in [
            ("enabled = true", "enable = true"),
            ("reset_hour = 4", "reset_hour = 24"),
            ("Asia/Tokyo", "Asia/Tokio"),
            ("2026-10-01T15:00:00+09:00", "tomorrow"),
        ] {
            let text = SAMPLE.replace(from, to);
            assert!(StatusConfig::from_toml(&text).is_err(), "{to}");
        }
    }

    /// ゲーム内の日付は、タイムゾーンの時刻で reset_hour 時に変わる(4時の直前は前の日)。
    #[test]
    fn game_date_changes_at_reset_hour() {
        let game_day = StatusConfig::from_toml(SAMPLE).unwrap().game_day;
        let date_at = |time: &str| game_day.date_at(time.parse().unwrap()).to_string();

        assert_eq!(date_at("2026-10-06T03:59:59+09:00"), "2026-10-05");
        assert_eq!(date_at("2026-10-06T04:00:00+09:00"), "2026-10-06");
        assert_eq!(date_at("2026-10-06T23:59:59+09:00"), "2026-10-06");
    }

    /// プラットフォームの名前は小文字の4種類だけを受け付ける。
    #[test]
    fn platform_from_str() {
        assert_eq!("ios".parse(), Ok(Platform::Ios));
        assert_eq!("android".parse(), Ok(Platform::Android));
        assert_eq!("windows".parse(), Ok(Platform::Windows));
        assert_eq!("editor".parse(), Ok(Platform::Editor));
        assert_eq!("iOS".parse::<Platform>(), Err(()));
        assert_eq!("macos".parse::<Platform>(), Err(()));
    }
}
