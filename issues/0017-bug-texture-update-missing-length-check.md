# 公開 Texture::update_* がバッファ長未検証のまま FFI する

- Priority: High
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-texture-update-missing-length-check
- Polished:

## 目的

公開 API の `Texture::update_yuv` / `update_nv12` / `update_packed` が slice 長を検証せず SDL に渡す未定義動作経路を塞ぐ。

## 優先度根拠

`Texture` は `lib.rs` で公開されている。短すぎるバッファと大きな pitch の組み合わせで SDL が範囲外読みをしうる。メモリ安全性に直結するため High。

## 現状

`VideoPlayer` 経由の enqueue は `validate_*` で長さを検証するが、低レベル API の `Texture::update_*` は検証なしで `SDL_UpdateYUVTexture` / `SDL_UpdateNVTexture` / `SDL_UpdateTexture` にポインタと pitch だけを渡す。

### 対象箇所

- `src/texture.rs` の `update_yuv` / `update_nv12` / `update_packed`
- `src/lib.rs` の `Texture` 公開

## 設計方針

テクスチャの `height` / フォーマットに応じた最小バイト数を `checked_mul` で計算し、slice 長が不足なら `Error::invalid_argument` を返す。`new` でも width/height の正値を拒否する。

## 完了条件

- 各 `update_*` が長さ不足で `Err` を返し、FFI を呼ばない
- 正常長のバッファでは従来どおり更新できる
- 長さ不足の単体テストがある

## 解決方法

1. `update_yuv` / `update_nv12` / `update_packed` に最小長検証を追加する
2. 必要なら `Texture::new` に寸法検証を追加する
3. エラーパスの単体テストを追加する
