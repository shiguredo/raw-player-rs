# enqueue_video_pixel_buffer が対応フォーマットと偶数寸法を enqueue 時に検証しない

- Priority: High
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-pixel-buffer-enqueue-format-validation
- Polished: 2026-07-22

## 目的

`enqueue_video_pixel_buffer` が I420/NV12 以外や奇数寸法を受け付け、描画時まで失敗を遅延させないようにする。

## 優先度根拠

他の enqueue API（`validate_i420` / `validate_nv12`）は投入時点でフォーマット制約と偶数寸法を厳密検証する。PixelBuffer パスだけが緩く、非対応フォーマットは描画時 `Err` になるが、その時点で既にキューから `pop_front` 済みでフレームが失われる。closed 0015 で正値・`MAX_DIMENSION` と描画時のプレーン高さ／stride 検証は入ったが、フォーマットと偶数制約の enqueue 前倒しは残っている。

## 現状

`enqueue_video_pixel_buffer`（`src/video_player.rs` 654–682 行付近）は width/height の正値と `MAX_DIMENSION` のみ検証する。`VideoFormat` が I420/NV12 以外でも、奇数寸法でもキューに入る。

描画（`render_next_frame` で `pop_front` 後の `render_frame_internal`）では:

- I420/NV12 以外は `Err("PixelBuffer does not support format: ...")`
- 奇数寸法は `div_ceil(2)` で進み、他 API のような偶数拒否が無い

`y_pitch` / `uv_pitch` 引数は `VideoFrame` に保存されるが、PixelBuffer 描画では `PixelBufferLock::stride` を使うため **未使用**。

`examples/player.rs` は PixelBuffer 経路で YUY2 を format に渡しうる（修正後は enqueue `Err`）。

### 対象箇所

- `src/video_player.rs` の `enqueue_video_pixel_buffer`（および必要なら `validate_*` と同層の切り出し関数）
- 退行テスト: `tests/test_video_player.rs` または既存テストファイルへの追加（検証関数の単体）

### 関連

- closed 0015: 実プレーン高さ・stride は **描画時**検証（enqueue 時ロック二重を避ける）。本 issue はロック不要な format／偶数のみ enqueue 前倒し。0015 の描画時平面検証は維持する
- open 0020: I420 の V stride。本 issue では触らない
- open 0017: 公開 `Texture::update_*`。層が異なる

### 本 issue の範囲外

- 描画失敗時に pop 済みフレームをキューへ戻す等の喪失回復（残リスクとして、平面不足・lock 失敗・0020 等では pop 後 `Err` が残る）
- `y_pitch` / `uv_pitch` の下限検証や API 削除（未使用のまま。API コメントに「描画では CVPixelBuffer の実 stride を使う」と明記する程度は本修正に含めてよい）
- `examples/player.rs` の YUY2+PixelBuffer 追随（呼び出し側が対応 format を選ぶ。必要なら別 change）

## 設計方針

enqueue 時点で次を検証し、失敗時は `Error::invalid_argument`、キューに入れない。

1. 既存: 正値、`MAX_DIMENSION`
2. **追加:** `format` が `I420` または `NV12` のみ（`YUY2` / `Rgba` / `Bgra` は拒否）。メッセージは描画側と同趣旨（例: `"PixelBuffer does not support format: ..."`）
3. **追加:** I420/NV12 は `validate_i420` / `validate_nv12` と同じく **width と height の両方**が偶数（メッセージも既存に揃える）

検証は `PixelBufferRef::from_ptr` **より前**に置く（非 macOS でも format／偶数の Err を検証でき、無駄な CFRetain を避ける）。

実プレーンの高さ・stride 整合は closed 0015 どおり描画時のまま。

### 後方互換

以前は enqueue 成功・描画失敗だった非対応 format／奇数寸法が、enqueue `Err` になる。意図的な fail-fast（他 enqueue API との一貫性）。

## 完了条件

- 非 I420/NV12 は enqueue が `Err` を返し、キューに入らない
- I420/NV12 で奇数 width または奇数 height は同様
- I420/NV12 の偶数・正値・`MAX_DIMENSION` 内は検証上 Ok（実 CVPixelBuffer の成功 enqueue は macOS 任意）
- 上記の単体テストがある（下記）
- API コメントに pitch 引数が描画で未使用である旨がある

### テスト戦略

- 種別: 単体（意図的エラーパス。モック禁止）
- format／偶数はロック不要。検証を `from_ptr` より前（ ideally 純関数）に置けば **非 macOS でも Err を検証できる**
- 必須ケース: 非対応 format → Err、I420/NV12 奇数寸法 → Err、偶数正値 → 検証 Ok
- 実 CVPixelBuffer を使う成功パスは本 issue 必須にしない

## 解決方法

1. format＋偶数の検証を追加する（`validate_*` と同規則。切り出し関数推奨）
2. `from_ptr` より前に置く
3. `enqueue_video_pixel_buffer` の rustdoc に pitch 未使用を明記する
4. 単体テストを追加する
