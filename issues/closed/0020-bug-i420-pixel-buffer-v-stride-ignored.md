# I420 PixelBuffer 描画で V プレーン stride を見ない

- Priority: Medium
- Created: 2026-07-15
- Completed: 2026-07-23
- Model: Grok 4.5
- Branch: feature/fix-i420-pixel-buffer-v-stride-ignored
- Polished: 2026-07-22

## 目的

I420 の CVPixelBuffer 描画で V プレーンの stride を正しく取得・検証し、U/V で行送りが異なるバッファでも誤描画・範囲外読みを起こさないようにする。

## 優先度根拠

高さ不足や Y/U stride 不足は closed 0015 で検証されているが、V だけ `stride(1)` を流用している。`plane(2)` のスライス長は V 自身の stride 基準なのに、SDL には U の pitch を渡す。

- `stride_V < stride_U`: SDL がより大きい pitch で読む → **範囲外読みの候補**
- `stride_V > stride_U`: 長さは足りても行解釈がずれ → **色ずれ**

実カメラでは U/V 一致が多いが、API 上はプレーン別 stride があり、非対称はバグとして残す理由にならない。典型再現は稀なため Medium（メモリ安全性候補ではあるが、発生頻度で High にはしない）。

## 現状

I420 PixelBuffer パス（`src/video_player.rs` 1104–1131 行付近）:

- `y_pitch = lock.stride(0)?`
- `uv_pitch = lock.stride(1)?`（U のみ）
- V にも同じ `uv_pitch` を渡して `texture.update_yuv(..., v, uv_pitch)` する
- `lock.stride(2)` は未取得・未比較（リポジトリ全体でも `stride(2)` 呼び出しなし）

`PixelBufferLock::stride(index)` / `plane(index)` は既にプレーン index ごとに正しい。バグは呼び出し側のみ。

`Texture::update_yuv` はもともと `u_pitch` / `v_pitch` を別引数で受け取る（SDL も別 pitch 対応）。

### 対象箇所

- `src/video_player.rs` の `render_frame_internal` 内 I420 PixelBuffer 分岐
- 必要なら同ファイル内の pitch 下限検証の切り出し（単体テスト用）
- 退行: 切り出し検証の単体（非 macOS）

### 関連

- closed 0015: 描画時の高さ／stride 下限を入れたが、V は `stride(1)` 流用のまま残った。本 issue はその穴埋め
- open 0019: enqueue の format／偶数。描画時 V stride は触らない
- open 0017: 公開 `Texture::update_*` の長さ検証。層が異なる。**推奨実装順は 0017 先**（安全網）。0020 は 0017 非依存で完結する

### 本 issue の範囲外

- `FrameData::Planar` / `enqueue_video_i420_strided` の U/V 別 pitch API 化（公開 API は単一 `uv_pitch`）
- `pixel_buffer.rs` の `stride` API 変更（既に index 対応済み）
- 0017 / 0019 の作業本体

## 設計方針

**採用: U と V の stride が異なっても拒否しない。別 pitch で `update_yuv` に渡す。**

1. `u_pitch = lock.stride(1)?`、`v_pitch = lock.stride(2)?`
2. 下限は既存 U と同じ `half_w = (frame.width as usize).div_ceil(2)`。U/V それぞれ `>= half_w` でなければ `Err`
3. `texture.update_yuv(y, y_pitch, u, u_pitch, v, v_pitch)?`
4. 変数名は `uv_pitch` をやめ `u_pitch` / `v_pitch` にする（誤用再発防止）
5. エラーメッセージは Y/U/V を分けて出す（例: `Y=..., U=..., V=..., required Y>=..., U/V>={half_w}`）

0019 合併後は偶数保証で `half_w` と `width/2` が一致するが、本修正の式は描画パス既存の `div_ceil(2)` に揃える。

## 完了条件

- `stride(2)` を取得し、U/V いずれかが `half_w` 未満なら `Err`（メッセージに Y/U/V）
- U≠V でも別 pitch で更新し、不一致だけの理由では拒否しない
- 下記の単体テストがある

### テスト戦略

- 種別: 単体（意図的エラーパス。モック禁止）
- **必須（構造）:** コードレビューで `stride(2)`・U/V 別下限・別 pitch 渡しが固定されていること
- **必須（非 macOS）:** pitch 下限検証を純関数または `video_player.rs` 末尾の `#[cfg(test)]` から叩ける形に切り出し、少なくとも
  - `u_pitch < half_w` → `Err`
  - `v_pitch < half_w` → `Err`
  - `u_pitch`/`v_pitch` が異なり、どちらも `>= half_w` → `Ok`
  を検証する（Y 下限は既存どおり。plane height メッセージの分離は任意）
- **任意:** 実 CVPixelBuffer での色ずれ再現（macOS）
- **不可:** コメントで前提を書くだけを完了扱いにすること
- 0017 後は `update_yuv` 長検証が別 pitch の安全網にもなる（本 issue の必須代替にはしない）

## 解決方法

I420 PixelBuffer 描画で `stride(2)` を取得し、U/V を別々に下限検証して `update_yuv` に渡すようにした。

- `validate_i420_pixel_buffer_strides` を切り出し、U≠V でも下限を満たせば Ok とする
- エラーメッセージを Y/U/V 分離にした
- `src/video_player.rs` 内の単体テストで U/V 不足と不一致 Ok を検証した
