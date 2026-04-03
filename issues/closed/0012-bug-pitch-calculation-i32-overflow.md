# pitch 計算の i32 乗算がオーバーフローする

Created: 2026-04-03
Completed: 2026-04-03
Model: Opus 4.6

## 概要

validate 関数を通過した後の pitch 計算 (`width * 2`, `width * 4`) が `i32` 上でオーバーフローする可能性がある。

該当箇所:
- video_player.rs:186 — `validate_yuy2_strided` 内の `width * 2`
- video_player.rs:519 — `enqueue_video_yuy2` の `y_pitch: width * 2`
- video_player.rs:560 — `enqueue_video_rgba` の `y_pitch: width * 4`
- video_player.rs:580 — `enqueue_video_bgra` の `y_pitch: width * 4`
- video_player.rs:600 — `enqueue_video_bgra_owned` の `y_pitch: width * 4`

## 根拠

- `width` は外部入力由来の `i32`
- `i32::MAX` は約 21 億なので `width * 4` は `width > 536,870,911` でオーバーフロー
- debug build では panic、release build では wrap した不正値が SDL に渡される
- validate 関数は `usize` にキャストしてからサイズ検証するためオーバーフローを検出できない

## 対応方針

validate 関数の入口で `width` の上限を `i32::MAX / 4` に制限する。これにより後続の `width * 2` や `width * 4` が安全であることを保証する。

## 解決方法

`MAX_DIMENSION` 定数 (`i32::MAX / 4`) を定義し、全 validate 関数の入口で `width` と `height` が `MAX_DIMENSION` を超える場合にエラーを返すようにした。これにより後続の `width * 2` や `width * 4` の i32 乗算がオーバーフローしないことが保証される。
