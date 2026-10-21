# 映像のみ再生で再生時計が PTS からずれると描画が止まり続ける

- Created: 2026-10-21
- Completed: {YYYY-MM-DD}
- Branch: feature/fix-video-playback-clock
- Polished: {YYYY-MM-DD}

## 目的

`VideoPlayer` の映像のみ再生経路が、再生時計と到着 PTS のずれから復帰できず、描画が長時間
止まる問題を修正する。moq-sub で受信とデコードが続いているのに映像だけが止まる症状の
原因であり、利用者からはプレイヤーが壊れたように見える。

## 現状

- `render_next_frame` は `audio_started` が false のとき、最初の描画時に映像キュー先頭の
  `pts_us` を `SDL_GetTicksNS` の単調時計へ対応付け、以後はその対応を固定する
  (`video_only_started` / `video_start_time_ns` / `first_video_pts_us`)
- 表示時刻の判定幅は `sync_threshold_us` (40 ms)。早すぎるフレームは `repeated_frames` を
  増やして待ち、遅すぎるフレームは先頭から捨てる (`dropped_frames`)
- 映像キューの上限は `max_video_queue_size` (既定 5)。早すぎる側では上限超過で先頭が
  捨てられるため、表示時刻に近いフレームから先に失われる
- moq-sub は表示時刻を過ぎたフレームを選び、その時点の経過時間を PTS として
  `enqueue_video_*` に渡す。アプリ側の処理停滞や最初の `poll_events` の遅れで、時計と
  到着 PTS の差が 40 ms を超えて固定されると、全フレームが待機か破棄の対象になり描画が
  永久に止まる
- 実 SDL (dummy ドライバ、モック不使用) での再現: PTS に試験開始からの経過時間を使い、
  最初のフレーム投入から最初の `poll_events` までだけ 300 ms 待つと、描画は最初の 1 枚で
  止まり、その後 8 秒間描画が進まなかった (投入 227 / 描画 1 / 破棄 221 / キュー残 5)
- 時計が PTS より進む側でも同じことが起きる。描画開始後に 300 ms 停滞すると、以降の
  全フレームが遅延ドロップされ続ける
- 時計を再基準化する経路が無く、`repeated_frames` と `dropped_frames` が増えるだけで
  描画は再開しない

## 変更対象

- `src/video_player.rs` (`VideoPlayerInner` / `render_next_frame` / `enqueue_frame` /
  `stop` / `drain_video` / `VideoPlayerStats`)
- `tests/test_video_player.rs`
- `CHANGES.md`

## 設計方針

- 映像のみ経路 (`audio_started` が false) で「描画できない状態」が `RESYNC_TIMEOUT_US`
  (1 秒) 継続したら、キューを最新 1 枚に絞り、その `pts_us` に `video_start_time_ns` と
  `first_video_pts_us` を再基準化して即座に描画する (`resync_to_latest_frame`)。ライブ
  再生で遅れを取り戻すため、古いフレームは間引く
- 未来の PTS が正当な先行バッファである場合に復帰が誤発動しないよう、復帰は「描画
  できない状態の継続時間」だけで判定する。遅延ドロップと早すぎる待ちで
  `blocked_since_ns` を設定し、描画できたらクリアする。フレーム未到着の通常の待機では
  設定しない
- pause / play / stop / drain_video では `blocked_since_ns` をクリアし、pause 中に
  経過した時間で誤って復帰しないようにする
- 音声が開始している経路では音声時計を正とし、時計の再基準化はしない (A/V 同期を
  壊さない)
- 破棄の理由を区別できるよう、`VideoPlayerStats` に `dropped_frames_queue_full` /
  `dropped_frames_late` / `dropped_frames_resync` と `resync_count` を追加する。既存の
  `dropped_frames` は合計として維持する
- 復帰の発動をテストで固定する。報告の再現手順 (最初の投入から 300 ms 待って poll) を
  そのまま使い、両方向のずれから描画が継続することを確認する

## 完了条件

- 最初のフレーム投入から 300 ms 待って `poll_events` した後も描画が継続し、
  `resync_count` が 1 以上になること
- 再生開始後に 300 ms 停滞した場合も、以降の描画が継続すること
- 正常な 30 fps 相当の再生では `resync_count` が 0 のままであること
- `dropped_frames` が 3 つの理由別カウンタの合計と一致すること
- `cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` /
  `cargo test -p raw_player` が通ること

## 解決方法

1. `src/video_player.rs` に `RESYNC_TIMEOUT_US` (1_000_000) を追加し、`VideoPlayerInner`
   に `blocked_since_ns` / `resync_count` / 理由別ドロップカウンタを追加する
2. `render_next_frame` の映像のみ経路で、`blocked_since_ns` から 1 秒以上経過していれば
   `resync_to_latest_frame` を呼ぶ。この関数はキューの最後の 1 枚以外を
   `dropped_frames_resync` として捨て、最新フレームの PTS に時計を再基準化して描画する
3. `enqueue_frame` のキュー上限超過を `dropped_frames_queue_full`、遅延ドロップを
   `dropped_frames_late` に数え、`dropped_frames` は合計として維持する。`stats()` で新
   フィールドを返し、`stop()` では既存カウンタと同様にリセットする
4. `tests/test_video_player.rs` に、時計が PTS より遅れる場合・進む場合の復帰、正常な
   30 fps で復帰しないこと、`drain_video` が復帰状態をクリアすること、キュー上限超過が
   理由別カウンタに入ることを追加する。いずれも実 SDL の dummy ドライバを使い、モック・
   スタブは使わない
5. 修正前のコードで復帰 2 ケースが描画 1 枚で失敗することを確認してから実装した。修正後
   は `cargo test -p raw_player` が全テスト成功 (test_video_player 11 本、約 11 秒)
6. `CHANGES.md` の `## develop` に [ADD] と [FIX] を追記する

moq-sub 側のログと表示実績の修正は moqt-rs 側で行う。この issue は raw_player 側の修正に
限定する。
