# DOCS_RS ダミー bindings が不完全で cargo check が失敗する

- Priority: Medium
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-docs-rs-dummy-bindings-incomplete
- Polished:

## 目的

`DOCS_RS=1` 時に出力するダミー `bindings.rs` を、クレートが参照する記号に揃えて型検査が通るようにする。

## 優先度根拠

docs.rs / CI の `cargo doc` 経路は現状通ることがあるが、`DOCS_RS=1 cargo check` は数十件の名前解決エラーになる。ダミーと実装の乖離は将来の rustc / docs.rs 変更で壊れる。

## 現状

`build.rs` は `DOCS_RS` 設定時に最小限の型・イベント定数だけを書き出して return する。一方本体は `SDL_Init` / `SDL_CreateWindow` / ピクセルフォーマット定数 / `SDL_Event` の `key`・`window` フィールドなどを参照する。

確認結果の例:

- `DOCS_RS=1 cargo check` が多数の `E0425` / `E0609` で失敗する
- ダミーの `SDL_Event` は `type_` と `_pad` のみで `Default` 派生も `[u8; 124]` で失敗しうる

### 対象箇所

- `build.rs` の `DOCS_RS` 分岐
- `src/` 全体の `ffi::` 参照

## 設計方針

ソースが名前解決する定数・関数スタブ・`SDL_Event` フィールドをダミーに網羅する。機械列挙できるなら build 時チェックやコメントで同期方針を残す。

## 完了条件

- `DOCS_RS=1 cargo check` が成功する
- `DOCS_RS=1 cargo doc --no-deps` が成功する
- CI の docs-rs ジョブが引き続き成功する

## 解決方法

1. `src/` の `ffi::` 参照を列挙し、ダミーに不足分を追加する
2. `SDL_Event` をキー／ウィンドウアクセス可能な形にする
3. ローカルで `DOCS_RS=1 cargo check` / `cargo doc` を確認する
