-- プレイヤーごとに、冪等性の対象の RPC の最新1件のレスポンスを持つ(設計書 4.3)。
-- 基盤はアプリの players テーブルを知らないので、外部キーは張らない
CREATE TABLE last_responses (
    player_id  BINARY(16) NOT NULL,
    request_id VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NULL,
    method     VARCHAR(255) CHARACTER SET ascii COLLATE ascii_bin NULL,
    response   MEDIUMBLOB NULL,
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    PRIMARY KEY (player_id)
);
