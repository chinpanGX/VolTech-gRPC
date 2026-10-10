# サーバー コーディング規約

`Server/` 以下の Rust のコードに適用する。

## 1. 整形と静的解析

書き方の統一は、公式の自動整形ツール **rustfmt** と静的解析ツール **Clippy** に任せる。人の手で決めるスタイルのルールは持たない。

- rustfmt は既定の設定で使う(`rustfmt.toml` は置かない)
- Clippy は既定の lint で使い、**警告ゼロ**を保つ(`-D warnings`)

| コマンド | 内容 |
|---|---|
| `make fmt` | 整形する(`cargo fmt --all`) |
| `make fmt-check` | 整形済みかを確かめる(`cargo fmt --all --check`) |
| `make lint` | Clippy を実行する(`cargo clippy --workspace --all-targets -- -D warnings`) |

- エディタで保存時に rustfmt がかかるようにしておく(RustRover: Settings > Rust > Rustfmt の「Run rustfmt on Save」)
- プッシュする前に `make fmt-check` と `make lint` を通す
- lint をどうしても抑制するときは、範囲を最小にし、`reason` に理由を書く

```rust
#[expect(clippy::too_many_arguments, reason = "tonic の生成 trait のシグネチャに合わせるため")]
fn handle(/* ... */) {}
```

## 2. コメント

### 必ず書く: pub な関数の概要

`pub` な関数には、何をする関数かの概要を `///` で必ず書く。1行目は1文で要点を書き、必要なら空行を挟んで補足する。

```rust
/// ガチャを1回引き、ジェムを消費して排出物を付与する。
///
/// ジェムが足りないときは `AppError::InsufficientGems` を返す。
pub async fn draw(tx: &mut Transaction<'_, MySql>, player_id: i64) -> Result<DrawResult, AppError> {
```

### 必ず書く: テストで何を確かめるか

テスト関数には、何を確かめるテストかを `///` で必ず書く。「どういう条件で、どうなるか」を1文で書く。

```rust
/// ジェムが足りないと、FAILED_PRECONDITION + INSUFFICIENT_GEMS になり、ジェムは減らない。
#[tokio::test]
async fn draw_without_enough_gems_fails() {
```

### 書く: コードから読み取れない意図

コードを読んでも分からない「なぜ」は残す。

- なぜこの順番・この書き方にしているか(例: ロックの順番、デッドロックを避けるため)
- 外部の仕様・ライブラリの制約に合わせている箇所
- 一見おかしく見えるが、わざとそうしている箇所(避けた落とし穴)

```rust
// 再送が届いても最初の処理の完了を待てるように、業務データより先にプレイヤーの行をロックする
lock_player(&mut tx, player_id).await?;
```

### 書かない

- **設計書の写しや設計の説明**(「設計書 4.3 参照」、レイヤーの役割、全体の流れの説明など)。設計が変わるたびにコメントも直す必要が出て、放置されると嘘になる。設計は設計書に書く
- コードを読めば分かること(`// player_id で検索する` など)
- 変更履歴・作業者名(Git に残る)
- コメントアウトしたコード(消す。必要なら Git から戻す)
