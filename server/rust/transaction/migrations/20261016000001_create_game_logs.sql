-- ゲームログ(所持データの変化の履歴)。種別ごとにテーブルを分けず、種別で形の変わりうる値は body の JSON に入れる。
-- 基盤はアプリの players テーブルを知らないので、外部キーは張らない
CREATE TABLE game_logs (
    log_id     BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    player_id  BINARY(16) NOT NULL,
    request_id VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NULL,
    method     VARCHAR(255) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    kind       VARCHAR(64) NOT NULL,
    body       JSON NOT NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    PRIMARY KEY (log_id),
    -- 問い合わせで、プレイヤーの履歴を時刻の順に引く
    KEY idx_game_logs_player_created (player_id, created_at)
);
