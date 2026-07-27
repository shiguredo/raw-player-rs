# VideoPlayer の NotPlaying と映像キュー溢れの退行テストが無い

- Priority: Low
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/test-video-player-not-playing-queue-overflow
- Polished: 2026-07-23

## 目的

`VideoPlayer` の enqueue ガードのうち、まだ退行テストが無い次の 2 経路だけを固定する。

1. 再生開始後に pause した状態での映像 enqueue → `Error::NotPlaying`
2. `max_video_queue_size` 超過時の先頭破棄と `dropped_frames` 増加

## 優先度根拠

中核経路の大半（`new` / enqueue / play / pause / `poll_events`、映像のみ pause のクロック、PixelBuffer 検証、`AudioPlayer` 成功経路）は既存テストでカバー済み。残るのは上記 2 分岐のみで、いずれも本番で壊れやすい契約だが影響範囲は限定的なため Low。

## 現状

実装（`src/video_player.rs`）:

- `enqueue_frame`: `has_played && !playing` のとき `Err(Error::NotPlaying)`。初回 `play()` 前は `has_played == false` のため enqueue 可。`pause()` 後は拒否。`stop()` は `has_played = false` に戻すため再 enqueue 可
- 同関数内: `max_video_queue_size > 0` かつ `video_queue.len() >= max` なら先頭を `pop_front` し `dropped_frames += 1` してから push

既存テスト（カバー済み・本 issue の対象外）:

- `tests/test_video_player.rs`: 映像のみ / AV の pause・play と同期ドロップ非増、PixelBuffer validate
- `tests/test_audio_player.rs`: play / pause / stop 成功経路
- `pbt/tests/prop_video_frame.rs`: `validate_*` の PBT

未カバー:

- pause 後の映像 enqueue → `NotPlaying`
- キュー溢れによる `dropped_frames` / キュー長

### 対象箇所

- `src/video_player.rs` の `enqueue_frame`（公開 `enqueue_video_*` 経由）
- `tests/test_video_player.rs`（追記）

## 設計方針

- モック・スタブ禁止。`tests/common` の `acquire_sdl()`（dummy ドライバ + プロセス内直列化）を使う
- 実装変更は原則不要。テスト追加のみ。実装を触る場合はテスト失敗の正当な修正に限る
- AV 同期の意図的ドロップ／リピート、PBT リネーム、`AudioPlayer` の `NotPlaying`、疑似テスト削除は **範囲外**（後者は既に置換済み）

## 完了条件

`tests/test_video_player.rs` に次を検証するテストがある。

1. **NotPlaying**: `play()` → `pause()` のあと `enqueue_video_i420`（または同等の公開 enqueue）が `Err(Error::NotPlaying)`。`Error::InvalidArgument` / `Sdl` では不合格。初回 `play()` 前の enqueue 成功は本ケースの前提として使ってよい
2. **キュー溢れ**: `set_max_video_queue_size(n)`（`n >= 1`）のあと、溢れるまで enqueue し、`stats().dropped_frames` が溢れた枚数だけ増え、`stats().video_queue_size == n`。再生開始前でも可（`has_played` 前は NotPlaying にならない）

`CHANGES.md` の develop にエントリを追記する（テストのみなら `### misc` の `[UPDATE]`、実装修正が入る場合は種別に合わせる）。

## 解決方法

1. `tests/test_video_player.rs` に上記 2 ケースを追加する（既存ヘルパ `enqueue_black_i420` を流用してよい）
2. NotPlaying ケースは必ず一度 `play()` してから `pause()` する（`has_played` が立っていないと到達しない）
3. 溢れケースは同期ドロップと混同しないよう、`play()` 前に完結させるか、`max` を明示して溢れだけを見る
4. 範囲外（AV 同期観測、PBT 改名、Audio NotPlaying）には手を出さない
