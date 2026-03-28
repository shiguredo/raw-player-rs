# pause 中の入力を Ok(()) で黙殺している

Created: 2026-03-28
Model: Opus 4.6

## 概要

`enqueue_audio()` と `enqueue_frame()` は `has_played && !playing` (pause 中) のとき `Ok(())` を返してデータを破棄する。呼び出し側はキューできたと誤認し、制御フローが壊れる。

## 根拠

`src/audio_player.rs:106-108`:
```rust
if inner.has_played && !inner.playing {
    return Ok(());
}
```

`src/video_player.rs:925-927`:
```rust
if inner.has_played && !inner.playing {
    return Ok(());
}
```

`pause()` という API 名は一時停止を意味するが、実装はデータ破棄であり、`Ok(())` を返すため呼び出し側は成功と判断する。ライブ入力や一時停止中の先読みで確実にデータ欠落を起こす。

蓄積するかどうかは設計判断として別途議論できるが、少なくとも成功扱いで黙殺するのは API 契約として不適切。

## 修正方針

エラーを返すか、戻り値で破棄されたことを明示する。蓄積方針は別途検討する。
