# `put_data` 失敗時に音声チャンクがキューから失われる

Created: 2026-04-02
Model: Composer 2 Fast

## なぜ対応が必要か

`process_audio_queue` は `audio_queue.pop_front()` した後に `SDL_PutAudioStreamData`（`put_data`）を呼ぶ。`put_data` が `Err` を返すと関数全体が失敗するが、取り出したチャンクはキューに戻らず、同じ PCM は再送されない。無音欠落や不連続再生につながる。

## 根拠（コード）

`src/audio_player.rs` の `process_audio_queue`:

- `while let Some(chunk) = inner.audio_queue.pop_front()` で先に dequeue。
- `stream.put_data(&chunk.data)?` が失敗すると、その時点で return し、該当 `chunk` は失われる。

## 期待する修正の方向性

- 失敗時に `push_front` で戻す、または
- `put_data` 成功までチャンクを手元に保持する、または
- 仕様として欠落を許容するならドキュメントとエラー型で明示する。
