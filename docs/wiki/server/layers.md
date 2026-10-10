---
title: "レイヤー構成"
status: confirmed
area: server
related: [server/transaction]
proto: []
updated: 2026-10-11
---

# レイヤー構成

```
sandbox/server/gacha/src/     ドメインのクレート(sandbox のガチャの例。player も同じ形)
├── service/                   業務ロジック本体。&mut Transaction とコレクターを引数で受け取る
├── model/                     各エンティティのstruct。モデル自身の状態変化だけで完結する処理
│                              (経験値からレベルを計算する等)はここのimplブロックに置く
└── lib.rs                     他のクレートに見せる関数・型だけを pub にする

sandbox/server/api/src/
└── gacha.rs                   薄いgRPCハンドラー(tonic生成traitの実装)。Proto⇔ドメインの変換、
                               with_transaction() の呼び出し、Responseの組み立てだけを行う
```

- `api` → `service/` → `model/` の3層にする。`api` はアプリに1つのクレートとして置き、`service/` と `model/` はドメインのクレートの中に置く
- モデル自身で完結する処理は、別のディレクトリに分けず、その型のimplブロックに書く(Rustには `partial class` が無く、分けると型の定義とロジックが離れて追いにくい)
- **`service/` の書き込み系の関数は `&mut Transaction<'_, MySql>` を引数に要求する**。トランザクションを持たない呼び出し元からはコンパイルが通らず、冪等性の仕組み([with_transaction() と冪等性](transaction.md))を通さずに書き込むことを型で防ぐ
