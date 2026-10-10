CREATE TABLE devices (
    device_id   CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    secret_hash VARCHAR(255) NOT NULL,
    created_at  DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    PRIMARY KEY (device_id)
);
