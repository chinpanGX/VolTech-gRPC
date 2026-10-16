use jiff::civil::Date;
use proto::errors;
use tonic::{Request, Status};
use ulid::Ulid;
use voltech_grpc_auth::AuthContext;
use voltech_grpc_error::{Internal, failed_precondition};
use voltech_grpc_status::GameDate;

/// `AuthLayer` が入れた認証の情報を取り出す。
pub(crate) fn auth_context<T>(request: &Request<T>) -> Result<&AuthContext, Status> {
    // AuthLayer を付け忘れたときだけ無い。クライアントには原因が無いので INTERNAL にする
    request.extensions().get::<AuthContext>().ok_or_else(|| {
        Internal::msg("AuthContext is missing. Is AuthLayer applied to this service?").into()
    })
}

/// `ServerStatusLayer` が入れた、リクエストを受けたときのゲーム内の日付を取り出す。
pub(crate) fn game_date<T>(request: &Request<T>) -> Result<Date, Status> {
    // ServerStatusLayer を付け忘れたときだけ無い。クライアントには原因が無いので INTERNAL にする
    request
        .extensions()
        .get::<GameDate>()
        .map(|date| date.0)
        .ok_or_else(|| {
            Internal::msg("GameDate is missing. Is ServerStatusLayer applied to the server?").into()
        })
}

/// サインアップ済みのプレイヤーの ID を取り出す。まだなら PLAYER_NOT_FOUND にする。
pub(crate) fn player_id<T>(request: &Request<T>) -> Result<Ulid, Status> {
    let context = auth_context(request)?;
    if context.player_created {
        Ok(context.player_id)
    } else {
        Err(player_not_found())
    }
}

/// プレイヤーがまだ作られていないことを表す業務エラーを作る。
pub(crate) fn player_not_found() -> Status {
    failed_precondition(errors::PLAYER_NOT_FOUND, "player is not created yet")
}
