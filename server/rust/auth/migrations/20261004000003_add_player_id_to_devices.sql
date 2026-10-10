-- デバイス登録のときに player_id を発行し、デバイスとプレイヤーの対応をここで持つ(設計書 3.7)。
-- 既存のデバイスには player_id を決められないので、リリース前の今のうちに、開発・テストのデータを消してから足す
DELETE FROM access_tokens;
DELETE FROM devices;
ALTER TABLE devices
    ADD COLUMN player_id BINARY(16) NOT NULL AFTER device_id;
