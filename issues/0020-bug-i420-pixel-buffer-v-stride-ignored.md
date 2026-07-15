# I420 PixelBuffer 描画で V プレーン stride を見ない

- Priority: Medium
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-i420-pixel-buffer-v-stride-ignored
- Polished:

## 目的

I420 の CVPixelBuffer 描画で V プレーンの stride を正しく取得・検証し、U/V で行送りが異なるバッファでも誤描画しないようにする。

## 優先度根拠

高さ不足や Y/U stride 不足は既に検証されているが、V だけ `stride(1)` を流用している。実カメラバッファで U/V stride が異なる場合に色ずれ・範囲外読みの候補になる。

## 現状

I420 PixelBuffer パスは次のようになっている。

- `y_pitch = lock.stride(0)?`
- `uv_pitch = lock.stride(1)?`（U のみ）
- V にも同じ `uv_pitch` を渡して `texture.update_yuv(..., v, uv_pitch)` する
- `lock.stride(2)` は未取得・未比較

### 対象箇所

- `src/video_player.rs` の `render_frame_internal` 内 I420 PixelBuffer 分岐

## 設計方針

`v_pitch = lock.stride(2)?` を取得し、U/V それぞれの下限を検証したうえで `update_yuv` に渡す。U と V の stride 不一致を拒否するか、別 pitch で渡すかを実装時に決める（SDL API は U/V 別 pitch を受け取る）。

## 完了条件

- V プレーン stride を取得し、不足時は `Err`
- U/V で異なる正当な stride でも正しく更新できる、または明示的に拒否する
- 可能な範囲でテストがある

## 解決方法

1. `stride(2)` を取得して `update_yuv` の V pitch に使う
2. 下限検証を U/V それぞれに行う
3. 退行テストまたはコメントで前提を明記する
