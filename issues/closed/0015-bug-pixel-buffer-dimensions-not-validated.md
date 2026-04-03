# CVPixelBuffer 投入時に宣言サイズと実プレーンサイズの整合を検証していない

Created: 2026-04-03
Completed: 2026-04-03
Model: GPT-5.4

## 概要

`VideoPlayer::enqueue_video_pixel_buffer()` は `width` / `height` の正数チェックしか行わず、実際の `CVPixelBuffer` プレーンサイズと宣言サイズの整合を検証していない。

## 問題

他の投入 API (`enqueue_video_i420` / `enqueue_video_nv12` / `enqueue_video_yuy2` / `enqueue_video_rgba` / `enqueue_video_bgra`) は、SDL に渡す前に入力サイズを厳密に検証している。一方で `CVPixelBuffer` パスだけは、レンダリング時に `PixelBufferLock::plane()` から取得した生ポインタと `Texture::update_nv12()` / `Texture::update_yuv()` をそのまま SDL に渡している。

この状態で、呼び出し側が実際より大きい `width` / `height` を指定すると、SDL 側は宣言されたテクスチャサイズに基づいて各行を読み取ろうとする。Rust 側はポインタしか渡しておらず長さ情報は渡らないため、実プレーン長を超えて読み進めて未定義動作になりうる。クラッシュやセグメンテーションフォルトの候補になる。

## 対象箇所

- `src/video_player.rs:654-676` `enqueue_video_pixel_buffer`
- `src/video_player.rs:1073-1089` PixelBuffer 経由の `SDL_UpdateNVTexture` / `SDL_UpdateYUVTexture` 呼び出し
- `src/pixel_buffer.rs:126-157` 実プレーン長と stride の取得

## 再現条件

1. 実サイズより小さいプレーンを持つ `CVPixelBuffer` を用意する
2. `enqueue_video_pixel_buffer()` に対して、実サイズより大きい `width` / `height` を指定する
3. `poll_events()` で当該フレームを描画する

## 根拠

- `plane()` は `stride * plane_height` を使って Rust 側のスライス長を計算しているが、その長さは SDL には渡していない
- `render_frame_internal()` は `frame.width` / `frame.height` で作成したテクスチャに対して生ポインタと pitch だけを SDL に渡している
- したがって、宣言サイズと実プレーンサイズがずれた場合、SDL 側で範囲外読み取りが起こりうる

## 修正方針

- `enqueue_video_pixel_buffer()` で format ごとに実プレーンの高さ・stride・プレーン長を参照し、宣言された `width` / `height` と整合するかを検証する
- 少なくとも `I420` / `NV12` について、必要最小行数と必要最小バイト数を満たさない場合は `Err` を返す
- `width` / `height` にも他 API と同じ `MAX_DIMENSION` 制約を適用する

## 解決方法

- `enqueue_video_pixel_buffer()` に `MAX_DIMENSION` チェックを追加して他の enqueue API と一貫させた
- `PixelBufferLock` に `plane_height()` メソッドを追加し、CVPixelBuffer の実プレーン高さを取得可能にした
- レンダリング時 (`render_frame_internal`) の PixelBuffer パスで、SDL にデータを渡す前にプレーンの高さと stride がテクスチャの宣言サイズに対して十分かを検証するようにした
- enqueue 時ではなくレンダリング時に検証する方式を採用した (enqueue 時の検証はロックの二重取得コストが発生するため)
