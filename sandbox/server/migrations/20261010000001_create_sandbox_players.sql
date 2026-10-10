-- sandbox のプレイヤー。player_id はデバイス登録で発行した ULID(BINARY(16))
CREATE TABLE sandbox_players (
    player_id    BINARY(16) NOT NULL,
    counter      BIGINT NOT NULL DEFAULT 0,
    suspended_at DATETIME(3) NULL,
    created_at   DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    PRIMARY KEY (player_id)
);
