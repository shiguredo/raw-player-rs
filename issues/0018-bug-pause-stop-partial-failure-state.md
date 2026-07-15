# pause/stop/play の SDL 失敗時にフラグとデバイス状態が食い違う

- Priority: High
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-pause-stop-partial-failure-state
- Polished:

## 目的

`AudioPlayer` / `VideoPlayer` の `pause` / `stop` / `play` で SDL 操作が失敗したとき、内部フラグとデバイス状態が食い違わないようにする。

## 優先度根拠

部分失敗後は「停止扱いなのに音声が鳴り続ける」「映像は playing のまま音声だけ止まった」など制御不能な状態になりうる。AV プレイヤーの状態機械として許容できない。

## 現状

### AudioPlayer

- `pause`: `playing = false` のあと `stream.pause()?`。失敗するとフラグだけ停止
- `stop`: キュークリアと `has_played = false` のあと `pause`/`clear` が失敗すると、後続のカウンタリセットに到達しない
- `play`: `playing = true` のあと `process_audio_queue` / `resume` が失敗しうる

### VideoPlayer

- `pause` / `stop` は先に `self.audio.*()?` し、失敗時は映像側 `inner.playing` を更新しない

### 対象箇所

- `src/audio_player.rs` の `play` / `pause` / `stop`
- `src/video_player.rs` の `play` / `pause` / `stop`

## 設計方針

デバイス操作の成功後にフラグを更新する、または失敗時にロールバックする。`VideoPlayer` は audio/video を同一トランザクションとして扱う方針を選ぶ。

## 完了条件

- SDL 操作失敗時に「フラグだけ更新」「片方だけ更新」の状態が残らない
- 失敗時の契約（Err の意味・再試行可否）が rustdoc で分かる
- 可能な範囲で退行テストがある（失敗注入が難しければ状態遷移の文書化とコード構造で保証）

## 解決方法

1. `AudioPlayer` の pause/stop/play で更新順序を見直す
2. `VideoPlayer` 側も audio 失敗時の映像状態を一貫させる
3. 意図した契約を rustdoc に書く
