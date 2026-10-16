use sqlx::{MySql, Transaction};
use ulid::Ulid;
use voltech_grpc_error::Internal;

use crate::game_log::GameLog;

/// `Transactions::run` に渡す処理が受け取る、トランザクションとゲームログの書き込み先。
///
/// ゲームログはここからだけ書ける。書いたゲームログは、処理が成功してコミットしたときだけ残り、
/// ロールバックしたときと、再送で保存済みのレスポンスを返したとき(処理を呼ばない)には残らない。
pub struct TxContext {
    tx: Transaction<'static, MySql>,
    player_id: Option<Ulid>,
    read_only: bool,
    game_logs: Vec<GameLog>,
}

impl TxContext {
    pub(crate) fn new(
        tx: Transaction<'static, MySql>,
        player_id: Option<Ulid>,
        read_only: bool,
    ) -> Self {
        Self {
            tx,
            player_id,
            read_only,
            game_logs: Vec::new(),
        }
    }

    /// 処理を動かしているトランザクション。
    pub fn tx(&mut self) -> &mut Transaction<'static, MySql> {
        &mut self.tx
    }

    /// ゲームログを書く。コミットしたときに、`Transactions::new` で選んだ出力先へ書く。
    ///
    /// `NO_SIDE_EFFECTS` の RPC(読み取り専用のトランザクション)と、`AuthLayer` を付けていないサービスではエラーになる。
    #[track_caller]
    pub fn write_game_log(&mut self, log: GameLog) -> Result<(), Internal> {
        if self.read_only {
            return Err(Internal::msg(
                "game logs cannot be written in a NO_SIDE_EFFECTS RPC",
            ));
        }
        if self.player_id.is_none() {
            return Err(Internal::msg(
                "game logs need AuthContext. Is AuthLayer applied to this service?",
            ));
        }
        self.game_logs.push(log);
        Ok(())
    }

    pub(crate) fn into_parts(self) -> (Transaction<'static, MySql>, Option<(Ulid, Vec<GameLog>)>) {
        let logs = self
            .player_id
            .filter(|_| !self.game_logs.is_empty())
            .map(|player_id| (player_id, self.game_logs));
        (self.tx, logs)
    }
}
