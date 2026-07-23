# examples/player の I420 分割が短バッファでパニックする

- Priority: Medium
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-player-example-i420-uv-slice-panic
- Polished: 2026-07-23

## 目的

`examples/player.rs` の非 PixelBuffer I420 経路で、UV 連結バッファを長さ確認なしに U/V へスライスしてパニックする経路を塞ぐ。

## 優先度根拠

サンプルは利用者の入口。ライブラリの `validate_i420_strided` は **分割後** にしか働かないため、短すぎる `uv_data` では本体に届く前にプロセスが落ちる。本体のメモリ安全系（0017 等 High）とは層が異なり、正常なカメラ経路では `shiguredo_video_device` 側がサイズを担保することが多いため到達頻度は低い。体験破壊は残るが Medium。

## 現状

`enqueue_video_frame`（`examples/player.rs` 317–332 行付近）の I420 分岐（非 PixelBuffer フォールバック）:

```text
uv_h = height as usize / 2
u_plane_size = stride_uv as usize * uv_h
&uv_data[..u_plane_size]   /  &uv_data[u_plane_size..]
```

長さ確認なし。

| `uv_data.len()` | 実際 |
| --- | --- |
| `< u_plane_size` | **スライスでパニック**（本バグの核） |
| `u_plane_size <= len < 2*u_plane_size` | スライス成功（V 短い）→ validate が `Err` → main の `eprintln` で継続 |
| `== 2 * u_plane_size` | 正常系（Y も正しければ） |
| `> 2 * u_plane_size` | V＝残余 → validate が厳密一致で `Err` |

追加の危険:

- `stride_uv < 0` または `height < 0`: `as usize` が巨大値 → 乗算／スライスでパニックしうる（本体 validate 到達前）
- `height == 0` や小さい奇数で `uv_h == 0`: U スライスは空で **パニックしない** → 後段 validate の `Err`（本バグのパニック核ではない）

対照: NV12 は分割せず enqueue → 同種スライスパニック無し。PixelBuffer 経路も対象外。

同一関数内の既存パターン: `uv_data` 欠落は黙って `Ok(())`。Mjpeg/Unknown は `Once` 警告。main は enqueue `Err` を毎フレーム `eprintln`。

### 再現手順（概念）

1. 非 PixelBuffer・I420・連結 UV のコピー経路に入る（実カメラ正常系では稀。合成の短 `uv_data` でも可）
2. `uv_data.len() < stride_uv * (height / 2)`（いずれも正）となるデータを渡す
3. **実際**: `&uv_data[..u_plane_size]` でパニック
4. **期待**: パニックせず再生ループ継続

### 対象箇所

- `examples/player.rs` の `enqueue_video_frame` 内 I420 分岐（非 PixelBuffer）のみ

### 関連

- closed 0014: 起動時 `expect` → exit。本 issue は再生中フレームのスライス（フレーム単位は継続）
- open 0019 / 0020: `src/video_player.rs` の PixelBuffer。本 issue は examples のみ。`src/` は触らない
- 本体 `validate_i420_strided`: 分割後検証。本 issue は分割前のパニック回避のみ

## 設計方針

失敗時方針を次に固定する。

### スライス前ガード（パニック回避のみ）

次のいずれかのとき、スライスせず **別 `static Once` で日本語警告を 1 回出し `Ok(())` でスキップ**する（Unknown 用 `WARN` とは別。`Err` にはしない）。

1. `stride_uv < 0` または `height < 0`（負値の `as usize` ラップ回避。**`stride_uv == 0` / `height == 0` は含めない** — 現行どおりスライス後に validate `Err`）
2. `u_plane_size` を `usize` で安全に計算できない（両者が非負である確認後に `checked_mul`。例: `(stride_uv as usize).checked_mul((height as usize) / 2)`）
3. `uv_data.len() < u_plane_size`（**閾値は `u_plane_size`。`2 * u_plane_size` ではない**）

`height == 0` や `stride_uv == 0` で `u_plane_size == 0` となりパニックしないケースは、ガードに引っかからず従来どおり enqueue → validate の `Err` に任せる（経路を付け替えない）。

### ガード通過後

既存どおり `&uv_data[..u_plane_size]` と `&uv_data[u_plane_size..]` で enqueue。

- 中間長（`u_plane_size <= len < 2*u_plane_size`）・余り長: **従来どおり validate `Err` → main の `eprintln`。本 issue で変えない**
- V＝残余の切り方・余りの厳密扱いは範囲外

### 範囲外

- `src/`・NV12・PixelBuffer・Y プレーン長
- 中間長を Once スキップに前倒しすること（目的外のソフト検証）
- examples の自動テスト基盤の新設

### 後方互換

- 正当な I420 再生は不変
- `len < u_plane_size` および負の stride/height・乗算不能: 以前はパニック → 今後は Once 警告 + スキップ
- 中間長・余り長・`stride_uv == 0` / `height == 0`: 不変（validate `Err`）

## 完了条件

- `uv_data.len() < u_plane_size`、および `stride_uv < 0` / `height < 0` / 乗算不能でもプロセスがパニックしない
- 正当な I420（コピー経路）は従来どおり再生できる
- 上記ガード発火時は Once 警告が 1 回出る
- 中間長は従来どおり validate `Err` 経路のまま（Once スキップにしない）
- 自動テストは追加しない（差分レビューと手動／合成で確認）
- `CHANGES.md` develop に `[FIX]` と著者行 `- @ユーザー名`（issue 番号は書かない）

### テスト戦略

- 自動テストなし。モック禁止
- 手動: 正当 I420 が落ちないこと。可能なら短 `uv_data` でパニックしないこと
- 検証: 既存 CI（`cargo test --workspace --features source-build` 等）が壊れないこと

### 警告文面（例）

`警告: I420 の UV データが短すぎるか stride が不正なためフレームをスキップします`（既存の日本語 `eprintln` に合わせる）

## 解決方法

1. I420 分岐で上記ガード（負値・`checked_mul`・`len < u_plane_size`）をスライス前に入れる
2. 失敗時は別 `Once` 警告 + `return Ok(())`
3. 成功時のみ既存スライス + `enqueue_video_i420_strided`
4. `CHANGES.md` に `[FIX]` を追記する
5. `src/` は変更しない
