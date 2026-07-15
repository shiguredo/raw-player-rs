# 映像のみ再生で pause/resume するとフレームが大量ドロップする

- Priority: High
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-video-only-pause-clock-skew
- Polished:

## 目的

音声未開始（映像のみ）の再生経路で、一時停止中の実時間がマスタークロックに混入して再開直後にフレームが大量ドロップするバグを修正する。

## 優先度根拠

映像のみ再生は公式ユースケースであり、pause/resume は通常操作。再開直後にキューが空になるため再生が破綻する。コードレビューで致命的と判定した。

## 現状

映像のみ経路のクロックは `first_video_pts_us + (now - video_start_time_ns)` で計算される。

- `pause()` は `playing = false` にするだけで `video_start_time_ns` / `video_only_started` を触らない
- `play()` も再開時に映像クロックを補正しない
- `poll_events` は `playing` 中だけ `render_next_frame` を呼ぶが、再開後の初回で壁時計が進んだクロックと比較するため `diff < -sync_threshold_us` となりドロップする

音声あり経路はサンプル基準で止まるため、本問題は映像のみ経路に限定される。

### 対象箇所

- `src/video_player.rs` の `pause()` / `play()`
- `src/video_player.rs` の `render_next_frame` 内の映像のみクロック計算

## 設計方針

pause 中の経過時間をマスタークロックから除外する。resume 時に `video_start_time_ns` へ停止時間を加算する、または pause 時に `video_only_started` を落として再開時に再アンカーする。

## 完了条件

- 映像のみ再生で数秒 pause したあと play しても、停止直前のフレーム付近から続き、大量ドロップが起きない
- 上記を退行防止するテストがある（SDL dummy 等）
- 音声あり経路の pause/resume 挙動を壊していない

## 解決方法

1. pause 開始時刻、または累積 pause 時間を `VideoPlayerInner` に保持する
2. resume 時に映像のみクロック基準を補正する
3. 映像のみ pause/resume の退行テストを追加する
