# サンプル player.rs が recoverable error で panic する

Created: 2026-04-03
Completed: 2026-04-03
Model: Opus 4.6

## 概要

`examples/player.rs` で `VideoCapture::new`、`VideoPlayer::new`、`video_capture.start()`、`player.play()` の失敗を `expect()` で処理しており、運用で普通に起こるエラーで即 panic する。

## 問題

サンプルは「お手本」として公開される。デバイス未接続や SDL 初期化失敗のような運用で普通に起こる条件で panic するのは不適切。`AudioCapture` は既に graceful に処理しているのに、他の箇所は `expect()` で一貫性がない。

## 対象箇所

- `examples/player.rs:392` — `VideoCapture::new(...).expect(...)`
- `examples/player.rs:413-414` — `VideoPlayer::new(...).expect(...)`
- `examples/player.rs:431-433` — `video_capture.start().expect(...)`
- `examples/player.rs:442` — `player.play().expect(...)`

## 根拠

CLAUDE.md に「サンプルはお手本なので性能と堅牢性を両立させること」と明記されている。

## 修正方針

- `expect()` を `eprintln!` + `process::exit(1)` に置き換える
- `main()` の戻り値を変更せず、各箇所で明示的にエラーメッセージを出力して終了する

## 解決方法

- `VideoCapture::new` を `match` で受けて `Err` 時に `eprintln!` + `process::exit(1)` とした
- `VideoPlayer::new` を `match` で受けて同様に処理した
- `video_capture.start()` を `if let Err(e)` で受けて同様に処理した
- `player.play()` を `if let Err(e)` で受けて同様に処理した
- `AudioCapture` と同じ graceful なエラーハンドリングパターンに統一した
