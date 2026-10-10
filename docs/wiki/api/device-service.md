---
title: "API: DeviceService(デバイス登録・トークンの発行)"
status: current
area: api
sections: []
related: [api/auth, api/metadata, api/errors, server/auth, client/auth-account]
proto: [server/rust/auth/proto/voltech/v1/device.proto]
updated: 2026-10-10
---

# API: DeviceService(デバイス登録・トークンの発行)

基盤の `voltech-grpc-auth` クレートが持つ、デバイス登録とアクセストークンの発行の API(`voltech.v1.DeviceService`)。なぜこの形にしたかは [認証](auth.md) にある。

- 定義: `server/rust/auth/proto/voltech/v1/device.proto`
- サービス全体が `skip_auth` なので、`authorization` を付けずに呼ぶ
- RPC の説明とリクエスト・レスポンスの表は、`.proto` のコメントから生成している(`make wiki-gen`)。表を直すときは、このページではなく `.proto` のコメントを直す

## 流れ

1. **デバイス登録**(`Register`): クライアントが生成した `secret_key` を送り、`device_id` と `player_id` を受け取る
2. **トークンの発行**(`IssueToken`): `device_id` と `secret_key` を送り、アクセストークンを受け取る
3. **認証が必要な API**: メタデータ `authorization: Bearer <access_token>` を付けて呼ぶ([下](#認証が必要な-api-の呼び方))

クライアントは `device_id`・`secret_key`・`player_id` を端末に保存しておき、トークンが切れたら 2 からやり直す。`player_id` は ID だけで、プレイヤー(初期データ)はアプリのサインアップで作る。

## Register

<!-- generated:rpc voltech.v1.DeviceService/Register -->

`/voltech.v1.DeviceService/Register`

クライアントが生成した secret_key を登録し、device_id と player_id を発行する。player_id は ID だけで、プレイヤー(初期データ)はアプリのサインアップで作る。再送で別の device_id が発行されても、使われない行が残るだけで害はない。

| 認証 | 冪等性(`idempotency_level`) |
|---|---|
| 不要(`skip_auth`) | `IDEMPOTENT`(再送してよい) |

**リクエスト** `RegisterRequest`

| フィールド | 型 | 内容 |
|---|---|---|
| `secret_key` | string | クライアントが生成した秘密の値。16〜128 文字の、表示できる ASCII 文字(0x21〜0x7E)。登録した後は変えられないので、端末に device_id と一緒に保存する。 |

**レスポンス** `RegisterResponse`

| フィールド | 型 | 内容 |
|---|---|---|
| `device_id` | string | デバイスの ID(ULID。大文字の英数字 26 文字)。 |
| `player_id` | string | プレイヤーの ID(ULID)。サインアップの前から持てるので、問い合わせなどに表示できる。 |

<!-- /generated -->

- 呼ぶたびに新しい `device_id` と `player_id` を発行する。同じ `secret_key` で2回呼んでも、別のデバイスとして登録される
- `secret_key` はハッシュ(Argon2)にして保存する。サーバーからは取り出せないので、クライアントがなくしたら登録し直す

### `secret_key` の作り方

| 項目 | 決まり |
|---|---|
| 長さ | **16〜128 文字** |
| 使える文字 | **表示できる ASCII 文字**(`!`〜`~`、0x21〜0x7E)。空白・制御文字・ASCII 以外の文字は使えない |
| 推奨 | `Guid.NewGuid().ToString()`(ハイフン入り 36 文字。例: `3f2504e0-4f89-11d3-9a0c-0305e82c3301`) |

- サーバーは長さと文字しか確かめない。値のランダムさはクライアントの責任
- Postman で試すときは、適当な文字列でもよいが、16 文字以上にし、空白を入れない

### エラー

| エラー | 条件 |
|---|---|
| `INVALID_ARGUMENT` | `secret_key` の長さが範囲外、または使えない文字を含む |
| `INTERNAL` + `ERROR_CODE_INTERNAL_ERROR` | DB のエラーなど |

## IssueToken

<!-- generated:rpc voltech.v1.DeviceService/IssueToken -->

`/voltech.v1.DeviceService/IssueToken`

device_id と secret_key を検証し、アクセストークンを発行する。発行し直すと、そのデバイスの古いトークンは無効になる。

| 認証 | 冪等性(`idempotency_level`) |
|---|---|
| 不要(`skip_auth`) | `IDEMPOTENT`(再送してよい) |

**リクエスト** `IssueTokenRequest`

| フィールド | 型 | 内容 |
|---|---|---|
| `device_id` | string | Register で受け取った値。 |
| `secret_key` | string | Register で送った値。 |

**レスポンス** `IssueTokenResponse`

| フィールド | 型 | 内容 |
|---|---|---|
| `access_token` | string | アクセストークン(256 ビットの乱数を 16 進にした、小文字 64 文字)。認証が必要な API に、メタデータ authorization: Bearer &lt;access_token&gt; で付ける。 |
| `expires_at` | `google.protobuf.Timestamp` | 有効期限。秒単位(nanos は常に 0)。 |

<!-- /generated -->

- 有効期限は、既定で発行から **1 時間**(`DeviceServiceImpl::with_token_ttl` で変えられる)
- 有効なトークンは **1 デバイスにつき1つ**。発行し直すと、そのデバイスの古いトークンはすぐに使えなくなる。別のデバイスのトークンには影響しない
- 有効期限の前でも発行し直してよい

### エラー

| エラー | 条件 |
|---|---|
| `UNAUTHENTICATED` | `device_id` が登録されていない、または `secret_key` が違う(どちらかは区別しない)。`secret_key` の長さや文字が `Register` の決まりに合わなくても、`INVALID_ARGUMENT` ではなくこれになる |
| `INTERNAL` + `ERROR_CODE_INTERNAL_ERROR` | DB のエラーなど |

## 認証が必要な API の呼び方

`AuthLayer` を付けたサービスは、メタデータに次を付けて呼ぶ([メタデータ](metadata.md))。

```
authorization: Bearer <access_token>
```

- `Bearer` はトークンの種類を表す決まった文字列(OAuth 2.0 の Bearer トークンの書き方に合わせている)。`Bearer` と半角スペース1つの後にトークンを続ける
- `Bearer` は大文字小文字を区別する(`bearer`・`BEARER` は通らない)。トークンだけを送っても通らない
- 期限は `expires_at` を過ぎた時点で切れる

| エラー | 条件 |
|---|---|
| `UNAUTHENTICATED` | `authorization` が無い、`Bearer ` で始まらない、トークンが存在しない・期限切れ・発行し直されて古くなった |
| `FAILED_PRECONDITION` + `ERROR_CODE_ACCOUNT_SUSPENDED` | プレイヤーが BAN・アカウント停止中(`PlayerResolver` が `Suspended` を返した) |
| `INTERNAL` + `ERROR_CODE_INTERNAL_ERROR` | DB や `PlayerResolver` のエラー |

`UNAUTHENTICATED` を受け取ったクライアントは、`IssueToken` でトークンを取り直してから再送する。

エラーコード(`ERROR_CODE_*`)は `ErrorInfo.reason` に入る。`ErrorInfo.domain` は `framework`([エラー](errors.md))。
