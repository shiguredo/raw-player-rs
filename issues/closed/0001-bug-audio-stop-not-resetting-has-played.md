# AudioPlayer::stop() が has_played をリセットしない

Created: 2026-03-28
Completed: 2026-03-28
Model: Opus 4.6

## 概要

`AudioPlayer::stop()` が `has_played` を `false` に戻していないため、停止後に `enqueue_audio()` を呼ぶと `Ok(())` を返しつつ実データを破棄する。

## 根拠

`VideoPlayer::stop()` は `inner.has_played = false;` でリセットしている (`src/video_player.rs:682`) のに対し、`AudioPlayer::stop()` (`src/audio_player.rs:152-166`) はリセットしていない。

`enqueue_audio()` (`src/audio_player.rs:106-108`) は `has_played && !playing` のとき `Ok(())` を返してデータを破棄するため、`stop()` 後に音声データを投入しても全て無視される。`VideoPlayer::stop()` も内部で `AudioPlayer::stop()` を呼ぶため、映像だけ進み音声が永続的に欠落する。

## 再現手順

1. `AudioPlayer` を作成し `play()` を呼ぶ (`has_played = true`, `playing = true`)
2. `stop()` を呼ぶ (`playing = false`, `has_played` は `true` のまま)
3. `enqueue_audio()` を呼ぶ → `has_played && !playing` が `true` なので `Ok(())` で破棄される
4. `play()` を呼んでも音声データがキューにないため無音

## 修正方針

`AudioPlayer::stop()` に `inner.has_played = false;` を追加する。

## 解決方法

`AudioPlayer::stop()` に `inner.has_played = false;` を追加した。`VideoPlayer::stop()` と同様に、停止時に再生状態を完全に初期化するようにした。
