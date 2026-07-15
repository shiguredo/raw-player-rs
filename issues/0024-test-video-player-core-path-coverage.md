# VideoPlayer 本体と AV 同期経路の退行テストが不足している

- Priority: Medium
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/test-video-player-core-path-coverage
- Polished:

## 目的

`VideoPlayer` の状態機械・AV 同期・キュー制御など、公開 API の中核経路に退行テストを追加する。

## 優先度根拠

既存テストは主に `validate_*` の PBT と薄い単体に偏る。`poll_events` / `render_next_frame` / キュー溢れ / `NotPlaying` / `process_audio_queue` はほぼ未カバー。致命バグ修正後も退行を検知できない。

## 現状

- PBT: `prop_audio_player`（enqueue 検証）、`prop_video_frame`（実体は `video_player` の validate）
- 単体: `tests/test_error.rs`、`pixel_buffer` の長さ計算、`audio_player` の VecDeque だけの疑似テスト
- `VideoPlayer::new` / `enqueue_*` / `play` / `pause` / `stop` / `poll_events` / `render_next_frame` を呼ぶテストが無い
- Makefile に fuzz ターゲットがあるが `fuzz/` は存在しない（本 issue の必須範囲外。必要なら別 issue）

### 対象箇所

- `src/video_player.rs`
- `src/audio_player.rs`
- `tests/` / `pbt/tests/`

## 設計方針

モックは使わない。SDL dummy / 実 API で再現できるものだけを単体テストにする。PBT でできる入力空間（validate）は既存を拡張する。ファイル命名は `test_video_player.rs` / `prop_video_player.rs` に揃える。

## 完了条件

最低限次をカバーするテストがある。

- pause 後 enqueue が `NotPlaying`
- 映像キュー溢れで `dropped_frames` が増える
- AV 同期のドロップ／リピートが観測できる、または映像のみクロックの基本挙動
- `prop_video_frame` の命名を `prop_video_player` に合わせる、または validate モジュール分割後の名前に合わせる
- 本番経路を通らない疑似テストを削除または置換する

## 解決方法

1. `tests/test_video_player.rs` / `tests/test_audio_player.rs` を追加する
2. SDL_VIDEODRIVER=dummy 等で可能な経路を固定する
3. PBT ファイル名と validate カバレッジ（strided / MAX_DIMENSION）を拡充する
4. `audio_player` の VecDeque のみのテストを削除または実経路に置換する
