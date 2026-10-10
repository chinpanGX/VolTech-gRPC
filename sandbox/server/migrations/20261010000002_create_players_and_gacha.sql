-- ガチャのサンプルを sandbox に移したので、プレイヤーを sandbox_players から players に移す。
-- sandbox は手元の確認用で残すデータが無いので、移さずに作り直す。
DROP TABLE sandbox_players;

-- プレイヤー。player_id はデバイス登録で発行した ULID(BINARY(16))
CREATE TABLE players (
    player_id    BINARY(16) NOT NULL,
    gems         BIGINT NOT NULL DEFAULT 0,
    counter      BIGINT NOT NULL DEFAULT 0,
    suspended_at DATETIME(3) NULL,
    created_at   DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    PRIMARY KEY (player_id)
);

-- 所持しているアイテム
CREATE TABLE player_items (
    player_id BINARY(16) NOT NULL,
    item_id   INT NOT NULL,
    count     BIGINT NOT NULL,
    PRIMARY KEY (player_id, item_id),
    CONSTRAINT fk_player_items_player FOREIGN KEY (player_id) REFERENCES players (player_id)
);

-- 1日1回の無料ガチャを、最後に引いたゲーム内の日付。まだ一度も引いていないプレイヤーの行は無い
CREATE TABLE gacha_free_draws (
    player_id     BINARY(16) NOT NULL PRIMARY KEY,
    last_drawn_on DATE NOT NULL
);
