/// リクエストの処理中に起きた出来事(`sandbox/v1/notification.proto` の `Notification`)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notification {
    /// ガチャでレアのアイテムを手に入れた。
    RareItemObtained { item_id: i32 },
    /// サーバーが Ping を受け取った。
    PingReceived,
}

/// 1つのリクエストの間に起きた出来事を、起きた順に集める(通知のコレクター)。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NotificationCollector {
    events: Vec<Notification>,
}

impl NotificationCollector {
    /// 出来事を積む。
    pub fn push(&mut self, event: Notification) {
        self.events.push(event);
    }

    /// 積んだ出来事を、起きた順に返す。
    pub fn events(&self) -> &[Notification] {
        &self.events
    }
}
