-- sandbox のサーバー用の DB(基盤のテストが使う app とは分ける)。ボリュームを作るときに1回だけ流れる
CREATE DATABASE IF NOT EXISTS voltech_sandbox;
GRANT ALL PRIVILEGES ON voltech_sandbox.* TO 'app'@'%';
