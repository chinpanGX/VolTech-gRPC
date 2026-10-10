use player::notification::{Notification as Event, NotificationCollector};
use proto::sandbox::v1::notification::Event as ProtoEvent;
use proto::sandbox::v1::{Notification, PingReceived, RareItemObtained};

/// 集めた出来事を、起きた順にレスポンスの `notifications` にする。
pub(crate) fn to_proto(notifications: &NotificationCollector) -> Vec<Notification> {
    notifications
        .events()
        .iter()
        .map(|event| Notification {
            event: Some(match *event {
                Event::RareItemObtained { item_id } => {
                    ProtoEvent::RareItemObtained(RareItemObtained { item_id })
                }
                Event::PingReceived => ProtoEvent::PingReceived(PingReceived {}),
            }),
        })
        .collect()
}
