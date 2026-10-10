---
title: "エラー処理"
status: confirmed
area: client
related: [api/errors, client/auth-account, client/response-items]
proto: []
code: [Unity/com.chinpangx.voltech.grpc/Runtime/Client/Errors, Unity/com.chinpangx.voltech.grpc/Runtime/Client/ErrorHandling]
updated: 2026-10-11
---

# エラー処理

## 受け取ったエラーの変換

受け取ったエラーをエラーコード([エラー](../api/errors.md))に変換し、以降はコードだけで扱う。

| 受け取ったもの | エラーコード |
|---|---|
| `FAILED_PRECONDITION` + `ErrorInfo` | `reason` を `ErrorCode` に戻したもの(知らない名前なら `UNKNOWN_ERROR`。`ErrorInfo` が壊れていて読めなければ、無いものとして `UNKNOWN_ERROR`) |
| `INTERNAL` | `INTERNAL_ERROR`(`ErrorInfo` が無くても) |
| `UNAVAILABLE` / `DEADLINE_EXCEEDED` | `NETWORK_ERROR` |
| `UNAUTHENTICATED` | 再認証して1回だけ送り直す。それでも失敗したらエラー([認証とアカウント情報](auth-account.md)) |
| クライアント内部の例外(共通レスポンス項目の反映処理など) | `CLIENT_ERROR`([共通レスポンス項目の反映](response-items.md)) |
| 上記以外(`ErrorInfo` の無い `FAILED_PRECONDITION`、`UNIMPLEMENTED` 等) | `UNKNOWN_ERROR` |

- 存在しないメソッドを呼ぶと `UNIMPLEMENTED` になるので、業務上の「見つからない」と混ざらない
- 変換そのものが失敗したら(`IErrorPolicyProvider` の実装の例外など)、`CLIENT_ERROR` にする。変換で起きた例外を呼び出し側に投げない

- action は、`metadata["action"]` があればそれ、無いか知らない値なら既定の action(`ErrorPolicyProvider` が、`errors.toml` から生成した一覧と予約エラーで `Dictionary<ErrorCode, ErrorAction>` にしておく)
- 文言と action の取り出しは `IErrorPolicyProvider` のようなインターフェースの裏に置き、将来マスターから読む形にも差し替えられるようにする

## 呼び出し側への返し方

- **Result型(`ApiResult<T>`、`readonly struct`)**で返す。呼び出し側に例外は投げない
  - 失敗の多くは「共通の処理が済んだので抜けるだけ」で、毎回 `try/catch` を書くのは大げさ。業務エラーは想定内の結果で、例外的な出来事ではない
  - 失敗時に `Value` を読んだら例外を投げる。成功の確認し忘れに、開発中に気づける
  - Grpc.Net.Client はOK以外で必ず `RpcException` を投げるので、性能の差は呼び出し側に投げ直すかどうかだけで、通信のコストと比べれば無視できる

## 自動リトライはしない

- 再送するかどうかは、必ずユーザーがエラーダイアログで選ぶ。`ApiCore` はエラーハンドラーを待ち、「再試行」なら同じRequestIDで送り直す
- 例外は `UNAUTHENTICATED` だけ([認証とアカウント情報](auth-account.md))
- Grpc.Net.Client の組み込みのリトライは有効にしない(設定しなければ無効)

## エラーが起きたときの流れ

1. 呼び出し側が「自分で処理する」と渡したエラーコード(`handledCodes`。省略できる)なら、ダイアログを出さずにすぐ返す
2. カスタムエラーハンドラーに登録されたコードなら、そのハンドラーが処理する
3. それ以外は、共通エラーハンドラー(`ICommonErrorHandler`)が処理する([エラー](../api/errors.md)の action の表のとおり)

- 予約コード(1〜999)は1と2を通らず、必ず共通エラーハンドラーが処理する。画面ごとにメンテナンスなどの扱いが変わらないようにするため
- ハンドラーは「再送する / 諦める」を返す。再送なら、順番待ちを持ったまま同じRequestIDで送り直す。諦めたら、呼び出し側に失敗を返す
- **エラーハンドラーの呼び出しは1つずつにする**(ダイアログは同時に1つだけ)。順番待ちに並ばないRPC(ポーリングなど)は同時にエラーになり得るため。後のエラーは前のダイアログが閉じるまで待ち、前のダイアログでタイトル・ホームへ戻った場合は、ダイアログを出さずに失敗として終わらせる

| action | ダイアログの後 | 呼び出し側に返るもの |
|---|---|---|
| Retry | 「再試行」なら同じRequestIDで再送する。「タイトルへ」ならタイトルに戻る | 再送で成功すれば成功。それ以外は失敗 |
| Stay | 画面に留まる | 失敗 |
| Home / Title | 先にセッションを終わらせ(送信中・順番待ちのリクエストを止めて捨てる)、順番待ちを解放してから、`ApiCore` が画面を遷移する | 失敗(続きの処理は動かさない) |
| Update | `ApiCore` がストアのページを開く。アプリは先へ進めない | 失敗 |

## タイトル・ホームに戻るとき

- **エラーハンドラーは動き(`ErrorDecision`)を返すだけで、画面の遷移はしない**。遷移は `ApiCore` が `IErrorNavigator` で行う
- `ApiCore` は次の順番で動く
  1. セッションを終わらせる。世代を進め、セッションごとの `CancellationTokenSource` を Cancel して作り直す(タイトルならトークンとアカウント情報も捨てる)
  2. 止めたリクエストは、送信中のものも順番待ちのものも、締め切りを待たずに抜けて「捨てた」失敗(`IsDiscarded`)になる。ダイアログは出さない
  3. 失敗したリクエストの順番待ちを解放する
  4. 画面を遷移する
- **なぜ**: 遷移が終わってからセッションを終わらせると、遷移先の画面(例: タイトルに着いてすぐサインイン)が遷移の途中に送ったリクエストが、古いセッションのものとして捨てられる。冪等性の対象のリクエストなら、失敗したリクエストが順番待ちを持ったままなので、止まったままになる(デッドロック)
- 遷移を決めた画面は、ほかの画面が送ったリクエストが残っているかを知らない。全部の通信を通している `ApiCore` が、セッションの区切りを一か所で引く
- 止めたリクエストも、サーバーでは処理が終わっていることがある。タイトルではサインインで全件を取り直すので食い違いは残らず、冪等性の対象のリクエストは RequestID で二重実行にならない
- 応答が届くのと止めるのがほぼ同時のときのために、応答を反映する前に世代も確かめる(キャンセルが間に合わなくても、古い応答は反映しない)
- 呼び出し側の `CancellationToken`(画面を閉じたら、その画面のリクエストだけ止める)は、v1 では受け取らない。冪等性の対象のリクエストを画面の都合で止めてよいかの整理が要るため、必要になったときにラッパーの引数に足す

## 画面ごとの個別の処理

- `handledCodes` で渡したコードは、ダイアログを出さずに返る。例: タイトル画面が、サインインの「プレイヤー未作成」を受け取ってサインアップへ進む
- 返した時点で順番待ちは解放される。もう一度呼ぶと新しいRequestIDになるが、エラーのレスポンスはサーバーに保存されないので問題ない
- どの画面から呼んでも同じ表示にしたいコード(例: ガチャ期間外の専用のポップアップ)は、カスタムエラーハンドラーで処理する

## カスタムエラーハンドラー

```csharp
[CustomErrorHandler("my-app", "GACHA_CLOSED", "GACHA_TICKET_SHORTAGE")] // 属性の形は未確定(未確定・保留 #24)
public sealed class GachaErrorHandler : ICustomErrorHandler
{
    public UniTask<ErrorDecision> HandleAsync(ApiError error) { ... }
}
```

- 機能ごとに複数作れる。担当するエラーコードは属性で書く。1つのエラーに対して動くハンドラーは1つだけ
- **担当コードの重複と、予約コードの指定は、基盤のRoslynアナライザーでコンパイルエラーにする**。アプリは属性を付けたハンドラーを書いてDIに登録するだけで、重複は書いた時点で気づける
- `ApiCore` も起動時に同じチェックをする(アナライザーが効いていないアセンブリがあっても気づけるように)
- v1では、カスタムエラーハンドラーを置くアセンブリを1つにする(アナライザーはコンパイル中のアセンブリしか見渡せない)
- `ApiCore` と同じルートのDIスコープに登録する(子のスコープの登録は `ApiCore` から見えない)

## 共通エラーハンドラー

- **基盤が既定の実装(`DefaultCommonErrorHandler`)を持つ**。action → ボタン → 挙動はどのアプリでも同じなので、メンテナンスやBANの扱いがアプリごとにずれない。Titleのときのセッションの破棄も、この中で行う
- アプリは、ダイアログの見た目(`IErrorDialogView`。`DefaultCommonErrorHandler` に渡す)と画面遷移(`IErrorNavigator`。`ApiCore` に渡す)だけを実装する。カスタムエラーハンドラーも同じダイアログを使い回せる

```csharp
public interface IErrorDialogView   // ダイアログの見た目(prefab)。押されたボタンを返す
{
    UniTask<ErrorDialogButton> ShowAsync(ApiError error, ErrorDialogButtons buttons);
}

public interface IErrorNavigator    // 画面遷移(ApiCore がセッションを終わらせてから呼ぶ)
{
    UniTask GoToTitleAsync();
    UniTask GoToHomeAsync();
    UniTask OpenStoreAsync();
}
```

- 既定の動きが合わないアプリは、`ICommonErrorHandler` ごと差し替えられる
- `ApiCore` はハンドラーから動き(再送する / 留まる / ホーム・タイトルに戻る / ストアを開く)だけを受け取る。ダイアログの表示には関わらず、画面遷移はセッションを終わらせてから自分で行う
- 基盤のエラー表示はすべてダイアログ(閉じるのにユーザーの操作が要る)にし、トースター(自動で消える表示)は持たない。トースターにしたいアプリは、カスタムエラーハンドラーで出し、すぐ「諦める」を返す(予約コードは担当できないので、通信エラーやメンテナンスがトースターになることは無い)。トースターで済むのは、action が Stay の業務エラー(例: ジェム不足)だけ。一時的な失敗は、再試行を選んでもらうためにダイアログが要る
