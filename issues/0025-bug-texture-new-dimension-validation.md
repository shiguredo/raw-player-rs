# Texture::new が非正・過大の width/height を拒否しない

- Priority: Medium
- Created: 2026-07-22
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-texture-new-dimension-validation
- Polished:

## 目的

公開 API の `Texture::new` / `new_yuv` が非正または過大な width/height を検証せず `SDL_CreateTexture` に渡す経路を塞ぐ。

## 優先度根拠

`Texture` は公開低レベル API である。非正・過大寸法のテクスチャが作れると、後段の `update_*` 契約が成立しにくくなる。enqueue 側は `validate_*` と `MAX_DIMENSION` で守られているが、`Texture::new` 直叩きは未ガード。メモリ安全性の主戦場は `update_*`（別 issue）だが、作成時ガードの欠落は独立した欠陥である。

## 現状

`src/texture.rs` の `Texture::new`（16–34 行付近）は `width` / `height` を見ずに `SDL_CreateTexture` へ渡す。`new_yuv` は `new(..., I420, ...)` への委譲のみ。成功時はそのまま `self.width` / `self.height` に保持する。

### 対象箇所

- `src/texture.rs` の `new` / `new_yuv`
- 必要なら `tests/test_texture.rs`（作成時エラーパス）

### 関連

- 公開 `Texture::update_*` の長さ・pitch 検証は別 issue（`update_*` 側）。本 issue は作成時の寸法ガードのみ
- closed 0012: enqueue 経路の `MAX_DIMENSION`。`Texture::new` は未カバー

## 設計方針

`SDL_CreateTexture` の前に次を検証し、失敗時は `Error::invalid_argument` を返す。

- `width <= 0` または `height <= 0` を拒否
- `width` / `height` が `MAX_DIMENSION`（`i32::MAX / 4`、closed 0012 と同値）を超える場合を拒否
- I420 / NV12 向けに偶数寸法を作成時に強制するかは、`update_*` 側の奇数拒否との役割分担を実装時に一文で固定する（作成時強制でも update 側拒否でもよいが、どちらか一方だけではテストが環境依存になる点に注意）

`new_yuv` は `new` への委譲のため、`new` の修正で追随する。

## 完了条件

- `width` / `height` が非正または過大のとき `Texture::new` が `Err` を返し、SDL を呼ばない
- 正当な寸法では従来どおり作成できる
- 上記の単体テストがある（`SDL_VIDEODRIVER=dummy` + `Window` / `Renderer`）

## 解決方法

1. `Texture::new` に非正・`MAX_DIMENSION` 検証を追加する（定数は `texture.rs` 局所で `video_player` と同値）
2. 必要なら I420/NV12 の偶数制約を作成時に追加する
3. `tests/test_texture.rs` に作成時エラーパスのケースを追加する
