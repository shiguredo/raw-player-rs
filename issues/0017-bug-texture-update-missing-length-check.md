# 公開 Texture::update_* がバッファ長未検証のまま FFI する

- Priority: High
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-texture-update-missing-length-check
- Polished: 2026-07-22

## 目的

公開 API の `Texture::update_yuv` / `update_nv12` / `update_packed` が slice 長と pitch を検証せず SDL に渡す未定義動作経路を塞ぐ。

## 優先度根拠

`Texture` は `lib.rs` で公開されている。短すぎるバッファと大きな pitch の組み合わせで SDL が範囲外読みをしうる。メモリ安全性に直結するため High。`VideoPlayer` の enqueue は `validate_*` で守られるが、低レベル `Texture` 直叩き経路は守られない。

## 現状

`src/texture.rs` の `update_yuv` / `update_nv12` / `update_packed` は、長さ・pitch 下限・`self.format` 一致を見ずに `SDL_UpdateYUVTexture` / `SDL_UpdateNVTexture` / `SDL_UpdateTexture` へポインタと pitch だけを渡す。`rect` 引数は常に null（テクスチャ全更新）のため、SDL はテクスチャの `height` 分行（chroma は `height/2` 行）を読む前提になる。

`VideoPlayer` 経由は enqueue 時の `validate_*`（厳密一致）と PixelBuffer 描画前の高さ／stride 下限で前段検証される。本バグの主戦場は公開 `Texture` を直接使う経路。

### 対象箇所

- `src/texture.rs` の `update_yuv` / `update_nv12` / `update_packed`（必要なら private ヘルパ）
- 退行テスト: `tests/test_texture.rs`（新規）

### 関連（本バグでは代替できない）

- closed 0012 / 0006 / 0015: enqueue や PixelBuffer 側の溢れ・plane 検証。公開 `Texture::update_*` は未カバー
- open 0019 / 0020: PixelBuffer enqueue・V stride。層が異なる
- open 0025: `Texture::new` の非正・過大寸法拒否（作成時ガード。本 issue とは別）

## 設計方針

FFI 前に次をすべて満たさない場合は `Error::invalid_argument` を返し、SDL を呼ばない。

### 前提

- `rect` は常に null＝テクスチャ全行更新
- 寸法は `self.width` / `self.height` を使う
- 長さ比較は **最小長以上を許可**（`len >= min`）。公開 `validate_*`（exact）は変更も流用もしない（式の参考のみ）
- `self.width <= 0` または `self.height <= 0` なら、長さ計算前に `invalid_argument`（`new` 未修正でも update 入口で守る）
- `self.width` / `self.height` が `MAX_DIMENSION`（`i32::MAX / 4`、closed 0012 と同値）を超える場合も Err。これにより pitch 下限の `width*2` / `width*4` 相当を **i32 直乗算せず**、`checked_mul` または `usize` 換算で安全に計算できる

### フォーマット・奇偶

- `self.format` とメソッドが一致しない場合は長さ計算前に Err
  - `update_yuv`: I420 のみ
  - `update_nv12`: NV12 のみ
  - `update_packed`: YUY2 / RGBA / BGRA のみ（I420/NV12 テクスチャへの `update_packed` も Err）
- I420 / NV12: `width` または `height` が奇数なら Err（`validate_*` と揃える。`height/2 == 0` の曖昧さを残さない）
- YUY2: `width` が奇数なら Err

### 最小長・pitch 下限

先に pitch が正かつ行バイト下限以上であることを保証し、その後 `usize` で `pitch.checked_mul(行数)` を計算する。負 pitch・乗算オーバーフローは Err。

行バイト下限の計算も `checked_mul`（例: YUY2 は `width.checked_mul(2)`、RGBA/BGRA は `width.checked_mul(4)`、I420 の chroma は `width / 2` を奇数拒否後に使用）。i32 の `width * 2` 直書きはしない。

| API | pitch 下限 | 最小長 |
| --- | --- | --- |
| `update_yuv` | `y_pitch >= width`、`u_pitch >= width/2`、`v_pitch >= width/2` | Y: `y_pitch * height`、U/V: 各 `*_pitch * (height/2)` |
| `update_nv12` | `y_pitch >= width`、`uv_pitch >= width` | Y: `y_pitch * height`、UV: `uv_pitch * (height/2)` |
| `update_packed` | YUY2: `pitch >= width*2`、RGBA/BGRA: `pitch >= width*4` | `pitch * height` |

表の `width*2` 等および最小長列の乗算は要件の略記である。実装は上文どおり `checked_mul` / `usize` 換算とし、i32 直乗算はしない。`MAX_DIMENSION` 定数は `texture.rs` 内に `video_player.rs` と同値（`i32::MAX / 4`）を局所定義してよい。

### 後方互換

- シグネチャは維持
- 短バッファ・不正 pitch・フォーマット不一致・奇数寸法・過大寸法: 以前は UB または SDL 由来 `Err` → 今後は FFI 前に `invalid_argument`
- 最小長を超えるバッファ: 引き続き成功（`>=`）
- 正常長: 挙動不変
- `VideoPlayer` 経路は二重検証になりうるが許容

## 完了条件

- 各 `update_*` が長さ不足・不正 pitch・フォーマット不一致・奇数寸法（該当 format）・非正／過大 `self` 寸法で `Err` を返し、FFI を呼ばない
- 最小長ちょうど、および最小長＋余白で従来どおり更新できる
- 上記の単体テストがある（下記）

### テスト戦略

- 種別: 単体のみ（意図的エラーパス。PBT 不要。モック禁止）
- ファイル: `tests/test_texture.rs`
- 環境: `SDL_VIDEODRIVER=dummy`。`Window` / `Renderer` / `Texture::new` は実 SDL API
- 必須ケース:
  - 各 `update_*`: 最小長 − 1 → `Err`、最小長ちょうど → `Ok`、最小長 ＋ 余白 → `Ok`
  - 負 pitch / pitch ＜ 行バイト下限 → `Err`
  - フォーマット不一致（I420 テクスチャへ `update_nv12` / `update_packed`、NV12 へ `update_yuv` / `update_packed`、YUY2 へ `update_yuv` 等）→ `Err`
  - I420/NV12 で奇数寸法のテクスチャを作れる場合は `update_*` が Err（作成時に拒否される場合は 0025 と合わせて担保）

## 解決方法

1. `update_*` に format・奇偶・非正／`MAX_DIMENSION`・pitch 下限・最小長（すべて overflow 安全）の検証を追加する
2. `tests/test_texture.rs` に上記ケースを追加する
