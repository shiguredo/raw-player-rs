# Texture::new が非正・過大の width/height を拒否しない

- Priority: Medium
- Created: 2026-07-22
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-texture-new-dimension-validation
- Polished: 2026-07-23

## 目的

公開 API の `Texture::new` / `new_yuv` が、非正または過大な width/height を Rust 側で検証せず `SDL_CreateTexture` に委譲する経路を塞ぎ、失敗時は FFI 前に `Error::InvalidArgument` を返すようにする。

## 優先度根拠

`Texture` は公開低レベル API である。enqueue 側は `validate_*` と `MAX_DIMENSION` で守られるが、`Texture::new` 直叩きは未ガード。作成時に非正・過大を FFI 前の `InvalidArgument` に揃え、保持寸法を `validate_*` / open 0017 の前提（`MAX_DIMENSION` 以下の正寸法）に閉じる。メモリ安全性の主戦場は `update_*`（0017、High）のため Medium。

## 現状

`src/texture.rs` の `Texture::new`（16–34 行付近）は `width` / `height` を見ずに `SDL_CreateTexture` へ渡す。`new_yuv` は `new(..., I420, ...)` への委譲のみ。成功時はそのまま `self.width` / `self.height` に保持する。受け付ける `VideoFormat` は I420 / NV12 / YUY2 / Rgba / Bgra の全種。

- **非正**（0 以下）: Rust 側検証なし。多くの環境では SDL が null を返し `Err(Error::Sdl)` になりうるが、種別・成否はドライバ依存
- **過大**（`MAX_DIMENSION` 超）: 検証なし。成否はドライバ依存。成功すれば巨大な `self.width` / `self.height` が保持される（実ドライバではこの閾値での成功は稀だが、Rust 側契約としては未定義のまま）

`VideoPlayer::create_texture`（`src/video_player.rs` 1197–1205 行付近）も内部で `Texture::new` を呼ぶ。通常の `enqueue_*` は `validate_*` 通過後のため、非正・`MAX_DIMENSION` 超は `create_texture` に届かない。本 issue の主効果は公開直叩き。

### 再現手順（概念）

1. `unsafe` で `SDL_VIDEODRIVER=dummy` と `SDL_AUDIODRIVER=dummy` を設定し、`init` → `Window::new("t", w, h)` → `Renderer::new(&window)` を用意する
2. `Texture::new(&renderer, VideoFormat::Rgba, 0, 16)` または `width = MAX_DIMENSION + 1` を呼ぶ
3. **期待（修正後）**: FFI 前に `Err(Error::InvalidArgument(_))`（メッセージは下記固定文言）

### 対象箇所

- `src/texture.rs` の `new`（`new_yuv` は委譲のため自動追随）
- `tests/test_texture.rs`（新規または追記。作成時エラーパス。上記テスト戦略の 1 関数規約に従う）

### 関連

- open 0017: `update_*` の長さ・pitch・format・奇偶。本 issue は作成時の非正・`MAX_DIMENSION` のみ。奇偶の「作成時拒否なら 0025 と合わせて担保」は本決定では適用しない。`width`/`height` は private で `new` 成功時のみ代入されるため、本 issue マージ後は公開 API で非正・過大の `self` を用意できない。0017 完了条件のうち「非正／過大 self で update が Err」は公開経路では検証不能になり、作成拒否（本 issue）で足りる（0017 側の文言整理は別途）
- open 0019: PixelBuffer のフォーマット・偶数前倒し。奇数 PixelBuffer → `create_texture` 経路は変えない
- closed 0012: `MAX_DIMENSION = i32::MAX / 4` の出所

## 設計方針

`SDL_CreateTexture` の前に次をこの順で検証し、失敗時は `Error::invalid_argument` を返して SDL を呼ばない。

1. `width <= 0` または `height <= 0` を拒否
2. `width > MAX_DIMENSION` または `height > MAX_DIMENSION` を拒否
3. 上記を通過したときだけ `SDL_CreateTexture` を呼ぶ

### 定数・メッセージ

- `texture.rs` 局所: `const MAX_DIMENSION: i32 = i32::MAX / 4;`（`video_player.rs` と同値。公開しない。コメントは既存 validate 側と同様に理由のみ書く: 「`width * 4` の i32 溢れを防ぐ上限。公開 Texture の保持寸法を validate / update 前提に揃える」。0017 が先に同定数を入れている場合は二重定義せず流用）
- メッセージは `validate_*` と同一
  - 非正: `"width and height must be positive"`
  - 過大: `"dimensions too large: {width}x{height} (max {MAX_DIMENSION})"`

### 本 issue の範囲外（明示）

- **偶奇制約は作成時に行わない。** I420/NV12 の奇数 width/height、YUY2 の奇数 width は 0017 の `update_*` が拒否。RGBA/BGRA および YUY2 の奇数 height は現行どおり作成可能
- `update_*` 本体（0017）
- `MAX_DIMENSION` のクレート共通化（リファクタ）

### 後方互換

- シグネチャは維持
- 非正・過大: 以前は SDL 由来 `Err(Sdl)`（または環境依存）→ 今後は FFI 前に `InvalidArgument`
- 正当寸法の成功経路は不変
- `VideoPlayer::create_texture` は二重ガードになりうるが、enqueue 通過後は成功経路不変

## 完了条件

- 非正のとき: `Error::InvalidArgument` かつ `err.message() == "width and height must be positive"`。`Error::Sdl` なら不合格
- 過大のとき: `Error::InvalidArgument` かつ `err.message()` が `"dimensions too large: {w}x{h} (max {MAX_DIMENSION})"` 形式。`Error::Sdl` なら不合格
- 正当な正寸法かつ `MAX_DIMENSION` 以下: 作成成功、`width()` / `height()` / `format()` がリクエストと一致
- `tests/test_texture.rs` に上記を検証するテストがある
- `CHANGES.md` の develop に `[FIX]` を追記する（エントリ次行に `- @ユーザー名`）

### テスト戦略

- 種別: 単体のみ（PBT 不要。モック・スタブ禁止）。テストコメント・`expect` 文言は日本語（AGENTS）
- ファイル・関数: `tests/test_texture.rs`。本 issue 側は **必ず 1 つの `#[test]` にケースを列挙**する。0017 とファイルを共有する場合も同一関数に足す（別 `#[test]` の並列 `quit` を避ける）。0017 が先に複数 `#[test]` を置いている場合は、合流時に 1 関数へ畳んでから本 issue のケースを足す。0017 未追随の間は本 issue 単独の 1 関数で閉じる
- import 例: `use raw_player::{init, quit, Error, Renderer, Texture, VideoFormat, Window};`
- `Error` に `PartialEq` は無い。照合は `matches!(err, Error::InvalidArgument(_))` と `err.message()` の組み合わせ
- 検証コマンド（CI と同値）:
  - `cargo fmt --all --check`
  - `cargo test --workspace --features source-build`
  - `cargo clippy --workspace --features source-build -- -D warnings`
- 環境変数（edition 2024 のため `unsafe`。テストでは **常に両方**）:
  - `unsafe { std::env::set_var("SDL_VIDEODRIVER", "dummy"); }`
  - `unsafe { std::env::set_var("SDL_AUDIODRIVER", "dummy"); }`
- セットアップ: `init()` → `Window::new("texture-test", 64, 64)` → `Renderer::new(&window)`（`new_gpu` は使わない）。FFI 前 reject でもシグネチャ上 `&Renderer` が必要
- 破棄: README と同様に **明示 `drop`** する。例: 成功ケースでは `drop(texture); drop(renderer); drop(window);` の後に `unsafe { quit() }`。関数末尾の暗黙 Drop は `quit()` の **後**に走るため、順序の列挙だけでは不十分
- メッセージ照合: **`err.message()` のみ**（`Display` は接頭辞付きで不一致）
- 過大用定数: テスト側で `const MAX_DIMENSION: i32 = i32::MAX / 4;` を再定義（`+ 1` は i32 内）
- 必須ケース（共有 1 関数内の本 issue 分）:
  - 非正: `(0, 1)`, `(1, 0)`, `(-1, 16)`, `(16, -1)`, `(0, 0)` → 上記非正メッセージ（`i32::MIN` も負として含めてよい）
  - 過大（**両軸とも必須**）: `(MAX_DIMENSION + 1, 16)` と `(16, MAX_DIMENSION + 1)` → 上記過大メッセージ
  - 成功: RGBA `16x16`、I420 `16x16`、`new_yuv(16, 16)` → Ok かつ `width`/`height`/`format` 一致
  - `MAX_DIMENSION` ちょうど成功は必須にしない
  - 偶奇ケースは含めない
  - `new_yuv` の非正 1 ケース（委譲退行防止）

## 解決方法

1. `src/texture.rs` に局所 `MAX_DIMENSION` を定義し（0017 が先なら流用）、`Texture::new` の `SDL_CreateTexture` 直前に非正 → 過大の順で検証を入れる。公開メソッドの rustdoc に寸法契約（非正・過大は `InvalidArgument`）を一文足す
2. `tests/test_texture.rs` を新設（または 0017 が先なら **同一 `#[test]` に追記**）し、上記必須ケースを実装する。明示 `drop` 後に `unsafe { quit() }`
3. `CHANGES.md` develop に `[FIX]` と著者行を追記する
4. 偶奇・`update_*`・定数共通化には手を出さない
