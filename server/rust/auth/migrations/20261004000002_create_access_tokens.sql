CREATE TABLE access_tokens (
    device_id  CHAR(26) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    token      CHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
    expires_at DATETIME(3) NOT NULL,
    PRIMARY KEY (device_id),
    UNIQUE KEY uk_access_tokens_token (token),
    CONSTRAINT fk_access_tokens_device FOREIGN KEY (device_id) REFERENCES devices (device_id)
);
