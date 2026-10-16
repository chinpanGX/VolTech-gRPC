use sqlx::{MySql, Transaction};
use ulid::Ulid;
use voltech_grpc_error::Internal;

use crate::game_log::{GameLog, GameLogs};

/// `Transactions::run` に渡す処理が受け取る、トランザクションとゲームログの書き込み先。
///
/// 書いたゲームログは、処理が成功してコミットしたときだけ残り、
/// ロールバックしたときと、再送で保存済みのレスポンスを返したとき(処理を呼ばない)には残らない。
pub struct TxContext {
    tx: Transaction<'static, MySql>,
    player_id: Option<Ulid>,
    read_only: bool,
    game_logs: GameLogs,
}

impl TxContext {
    pub(crate) fn new(
        tx: Transaction<'static, MySql>,
        player_id: Option<Ulid>,
        read_only: bool,
        game_logs: GameLogs,
    ) -> Self {
        Self {
            tx,
            player_id,
            read_only,
            game_logs,
        }
    }

    /// 処理を動かしているトランザクション。
    pub fn tx(&mut self) -> &mut Transaction<'static, MySql> {
        &mut self.tx
    }

    /// 認証したプレイヤーのゲームログを書く。コミットしたときに、`Transactions::new` で選んだ出力先へ書く。
    ///
    /// `NO_SIDE_EFFECTS` の RPC(読み取り専用のトランザクション)と、`AuthLayer` を付けていないサービスではエラーになる。
    /// 値の名前が使えないもの(`GameLog::field`)のときもエラーになる。
    #[track_caller]
    pub fn write_game_log(&mut self, log: GameLog) -> Result<(), Internal> {
        if self.read_only {
            return Err(Internal::msg(
                "game logs cannot be written in a NO_SIDE_EFFECTS RPC",
            ));
        }
        let Some(player_id) = self.player_id else {
            return Err(Internal::msg(
                "game logs need AuthContext. Is AuthLayer applied to this service?",
            ));
        };
        self.game_logs.write(player_id, log)
    }

    pub(crate) fn into_parts(self) -> (Transaction<'static, MySql>, GameLogs) {
        (self.tx, self.game_logs)
    }
}
