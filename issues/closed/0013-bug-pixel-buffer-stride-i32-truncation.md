# CVPixelBuffer の stride を i32 に無検証で切り詰めている

Created: 2026-04-03
Completed: 2026-04-03
Model: Opus 4.6

## 概要

`PixelBufferLock::stride()` が `CVPixelBufferGetBytesPerRowOfPlane()` の戻り値 (`usize`) を `as i32` で無検証に切り詰めている。

## 問題

`i32::MAX` を超える stride を持つバッファが渡された場合、負値や破損値になる。その値がそのまま `SDL_UpdateNVTexture` / `SDL_UpdateYUVTexture` に渡されるため、未定義動作の原因になる。

## 対象箇所

- `src/pixel_buffer.rs:150-151` (`stride()` メソッド)
- `src/video_player.rs:1079-1088` (呼び出し元)

## 根拠

この関数は unsafe FFI 境界に値を渡すパスの一部であり、壊れた値が SDL に渡ると未定義動作になる。`i32::try_from` による検証はコストゼロで堅牢性が向上する。

## 修正方針

- `stride()` の戻り値を `Result<i32>` に変更する
- 内部で `i32::try_from(...)` を使い、収まらない場合は `Err` を返す
- 呼び出し元で `?` を使ってエラーを伝播する

## 解決方法

- `PixelBufferLock::stride()` の戻り値を `i32` から `Result<i32>` に変更した
- 内部で `i32::try_from(stride)` を使い、`i32::MAX` を超える場合は `Error::invalid_argument` を返すようにした
- `src/video_player.rs` の呼び出し元 (NV12/I420 の pitch 取得箇所) で `?` によるエラー伝播を追加した
