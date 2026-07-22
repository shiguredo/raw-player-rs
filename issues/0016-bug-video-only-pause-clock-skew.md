# 映像のみ再生で pause 後の play でフレームが大量ドロップする

- Priority: High
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-video-only-pause-clock-skew
- Polished: 2026-07-22

## 目的

音声未開始（映像のみ）の再生経路で、一時停止中の実時間がマスタークロックに混入し、`play()` 再開直後にフレームが大量ドロップするバグを修正する。

## 優先度根拠

映像のみ再生は公式ユースケース（`skills/raw-player/SKILL.md`）であり、pause 後の `play()` は通常操作である。再開直後に映像キューが同期ドロップで空になると再生が破綻する。

## 現状

映像のみ経路のクロックは次式で計算される（`src/video_player.rs` の `render_next_frame`）。

```text
clock_us = first_video_pts_us + (SDL_GetTicksNS() - video_start_time_ns) / 1000
```

- `pause()`（713–717 行付近）は `playing = false` にするだけで `video_start_time_ns` / `video_only_started` / `first_video_pts_us` を触らない
- `play()`（698–709 行付近）も再開時に映像クロックを補正しない（`play_start_time_ns` の初回セットのみ）
- `poll_events` は `playing` 中だけ `render_next_frame` を呼ぶ（809–811 行付近）。再開後の初回比較で `diff < -sync_threshold_us`（既定 40_000 µs）となり、キュー内フレームが一括ドロップする（1026–1030 行付近）

音声あり経路（`audio_started == true`）は `AudioPlayer` のサンプル基準クロックのため、本問題は映像のみ経路（`audio_started == false`）に限定される。

### 再現条件

1. 音声を enqueue しない（または音声ストリーム未開始）
2. 映像フレームを複数 enqueue して `play()` し、少なくとも 1 回 `poll_events` で描画して `video_only_started == true` にする
3. `pause()` する（この時点で enqueue は `Error::NotPlaying` になるため、追加投入はできない）
4. `sync_threshold_us`（既定 40 ms）を超える実時間を置く（例: 200 ms 以上）
5. `play()` して `poll_events` する → `dropped_frames` が増え、キューが空に近づく／空になる

初回描画前（`video_only_started == false`）に pause した場合は、再開後の初回 `render_next_frame` が既存の再アンカーで基準を取り直すため、本バグの主経路ではない。

### 対象箇所

- `src/video_player.rs` の `pause()` / `play()`
- `src/video_player.rs` の `render_next_frame` 内の映像のみクロック計算
- 補正用フィールドを追加する場合のクリア経路: `stop()`（721–740 行付近）/ `drain_video()`（943–948 行付近）
- 退行テスト: `tests/test_video_player.rs`（新規）

## 設計方針

**採用: pause 中の経過時間を `video_start_time_ns` に加算して壁時計をずらす（タイムライン連続性を維持する）。**

`VideoPlayerInner` に `pause_started_ns: Option<u64>` を追加する（`None` = 補正待ちなし）。`play()` 再開時にその区間を `video_start_time_ns` へ加算する。`first_video_pts_us` は維持する。

### 採用しない案

pause 時に `video_only_started = false` へ落として再開時に再アンカーする案は採用しない。既存の初回開始・`stop` / `drain_video` と同じ経路になり、キュー先頭 PTS をメディア時刻ゼロに付け替えるため、「一時停止中の実時間を除外する」意図と一致しない。早期待ち（`diff > sync_threshold_us`）中の resume でも挙動が変わりうる。

### 状態遷移

補正の対象条件: `video_only_started == true` かつ `!audio.is_started()`（`video_only_started` は音声開始後も残りうるため、音声開始判定を必ず併用する）。

| 操作 | 期待する副作用 |
| --- | --- |
| `pause`（対象条件かつ `playing` true→false） | `pause_started_ns = Some(now)`。既に `Some` なら上書きしない |
| `pause`（対象条件外） | `pause_started_ns` は触らない |
| `play`（`pause_started_ns` が `Some`） | `video_start_time_ns` に経過分を加算してから `None` にする |
| `play`（`pause_started_ns` が `None`） | クロック補正なし |
| `stop` / `drain_video` | 既存リセットに加え `pause_started_ns = None` |

`video_start_time_ns` への加算は `now >= pause_started` を前提とする。逆転時は補正をスキップして `pause_started_ns` だけ `None` にする（巨大な unsigned 引き算で後続の全フレーム同期ドロップを起こさない）。

### `AudioPlayer` 失敗時（本 issue の契約）

現行どおり `self.audio.pause()?` / `self.audio.play()?` が成功したあとにだけ映像側（`playing`・`pause_started_ns`・`video_start_time_ns`）を更新する。audio が `Err` のときは開始時刻の記録も加算もクリアもしない。SDL 失敗時の audio/video 一貫性そのものは open issue 0018 の範囲。

### 本 issue の範囲外

- `play_start_time_ns` / `stats().elapsed_time_ms` が pause 中も進む既存挙動（統計用。同期ドロップとは別）
- `sync_threshold_us` の公開 setter 追加
- open issue 0018（pause/stop/play の SDL 失敗時の状態一貫性）
- open issue 0024（VideoPlayer 核心の横断カバレッジ）。本 issue の退行は下記最小セットに限定する

## 完了条件

- 映像のみ再生で `sync_threshold_us` を超える pause のあと `play()` しても、同期ドロップでキューが空にならない
- 複数回の pause / play でも skew が蓄積せず、同様に同期ドロップが増えない
- 音声あり経路の pause / play で、本修正による同期ドロップ増が起きない
- 上記を退行防止する単体テストがある

### テスト戦略

- 種別: **単体のみ**（壁時計と状態機械の退行は PBT 不向き。モック禁止）
- ファイル: `tests/test_video_player.rs`
- 環境: `SDL_VIDEODRIVER=dummy`（必要なら `SDL_AUDIODRIVER=dummy`）
- 映像のみ退行（必須）:
  1. `VideoPlayer::new` → `set_vsync(0)` → `set_max_video_queue_size` を十分大きくする（キュー溢れと同期ドロップを混同しない）
  2. I420 を PTS 間隔 33333 µs で N 枚（例: 10）enqueue → `play` → `poll_events` で少なくとも 1 枚描画
  3. 描画用 `poll_events` の直後に（間に sleep や重い処理を入れず）`dropped0` / `q0` を記録し、すぐ `pause` する（`q0 >= 1`）。描画〜pause 間に `sync_threshold_us` 超の実時間が空くと、pause 補正の対象外の遅れとして再開後に同期ドロップしうる
  4. 200 ms sleep → `play` → すぐに `poll_events`（ここも 40 ms を超えにくくする）
  5. 断言: `stats().dropped_frames - dropped0 == 0`、`stats().video_queue_size > 0`
  - 注: `stats().video_pts_us`（`last_video_pts_us`）は同期ドロップでは更新されないため、連続性の主指標にしない
  - 環境変数 `SDL_VIDEODRIVER=dummy`（必要なら `SDL_AUDIODRIVER=dummy`）は、既存の linux 単体と同様にテスト内で設定してよい
- 複数回 pause / play: 上記を 2 回繰り返し、毎回 `dropped_frames` 増分が 0
- 音声あり非退行: 音声を enqueue したうえで同様に pause / play し、`dropped_frames` 増分が 0（または修正前と同程度で悪化しないこと）を確認する
- `play()` 側の補正条件は `pause_started_ns` の有無のみとする（`is_started` を再判定してクリアをスキップしない）。記録条件の `!is_started()` は `pause()` 側だけ

## 解決方法

1. `VideoPlayerInner` に `pause_started_ns: Option<u64>` を追加し、初期化で `None` にする（`None` = 補正待ちなし）
2. `pause()`: `audio.pause()?` 成功後、対象条件（`video_only_started && !self.audio.is_started()`）かつ `playing` true→false かつ `pause_started_ns` が `None` のときだけ `Some(now)` を入れる
3. `play()`: `audio.play()?` 成功後、`pause_started_ns` が `Some(t)` なら `now >= t` のときだけ `video_start_time_ns += now - t` とし、`is_started` の再判定はせずいずれにせよ `None` に戻す
4. `stop()` / `drain_video()` でも `pause_started_ns = None` にする
5. `tests/test_video_player.rs` に上記テスト戦略の単体テストを追加する（`VideoPlayer::new` 前に `SDL_VIDEODRIVER=dummy` 等をテスト内で設定してよい）
