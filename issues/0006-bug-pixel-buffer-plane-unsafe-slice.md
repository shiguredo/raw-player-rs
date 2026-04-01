# `PixelBufferLock::plane` が `from_raw_parts` の前提をコード上保証していない

Created: 2026-04-02
Model: Composer 2 Fast

## なぜ対応が必要か

`CVPixelBufferGetBaseAddressOfPlane` がヌルを返す場合、または `stride * height` が実バッファ長と一致しない・乗算がオーバーフローする場合、`std::slice::from_raw_parts` は未定義動作になり得る。

## 根拠（コード）

`src/pixel_buffer.rs` の `PixelBufferLock::plane`:

- `ptr` のヌルチェックがない。
- `stride * height` の `checked_mul` がない。
- Core Video が返す値と実際に読みよいバイト数の再検証がない。

## 期待する修正の方向性

- `ptr.is_null()` のときはエラーにするか空スライス方針を決める（通常はエラー）。
- `stride` と `height` から長さを `checked_mul` で計算し、失敗時はエラー。
- 必要に応じてプレーンサイズを API で再検証する。
