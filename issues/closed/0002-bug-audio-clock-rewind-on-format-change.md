# 音声フォーマット変更時にクロック基準が更新されない

Created: 2026-03-28
Model: Opus 4.6

## 概要

`process_audio_queue()` はフォーマット変更時に `samples_written` を 0 にリセットするが、`audio_started` は `true` のままなので `first_pts_us` が更新されない。`get_audio_clock_us()` が古い基準点から時刻を再計算し、音声クロックが過去に巻き戻る。

## 根拠

フォーマット変更時の処理 (`src/audio_player.rs:337-350`):
- `samples_written = 0` にリセット
- `audio_started` は `true` のまま → `first_pts_us` は初回チャンクの値が残る

`get_audio_clock_us()` (`src/audio_player.rs:300-328`):
- `first_pts_us + played_samples * 1_000_000 / sample_rate` を返す
- フォーマット変更後は `played_samples` が 0 付近になるため、クロックが `first_pts_us` (古い値) まで巻き戻る

トラック切替やサンプルレート変更時に AV sync が壊れ、フレームドロップやフリーズを誘発する。

## 再現手順

1. 48kHz のオーディオデータを投入して再生開始
2. 途中で 44.1kHz のオーディオデータに切り替え
3. `get_audio_clock_us()` が最初のチャンクの PTS 付近に戻る

## 修正方針

フォーマット変更ブロック (`src/audio_player.rs:337-350`) 内で `first_pts_us` を新しいチャンクの `chunk.pts_us` に明示的に更新し、`samples_written = 0` と合わせて新しい再生系列の基準を設定する。「フォーマット変更 = 新しい再生系列」という意図をコード上で明示する。

## 解決方法

Completed: 2026-03-28

フォーマット変更ブロック内で `first_pts_us = chunk.pts_us` と `audio_started = true` を設定し、新しい再生系列のクロック基準を明示的に更新するようにした。
