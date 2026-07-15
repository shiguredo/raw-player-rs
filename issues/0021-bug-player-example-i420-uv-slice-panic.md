# examples/player の I420 分割が短バッファでパニックする

- Priority: Medium
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-player-example-i420-uv-slice-panic
- Polished:

## 目的

`examples/player.rs` が I420 の UV データを長さ確認なしにスライスしてパニックしないようにする。

## 優先度根拠

サンプルは利用者の入口。異常・想定外フォーマットのフレームでプロセスが落ちると、ライブラリ本体の堅牢性とは別に体験が壊れる。過去にサンプルの `expect` パニックは修正済みだが、スライス境界のパニックが残っている。

## 現状

I420 分岐で `u_plane_size = stride_uv * (height / 2)` を計算し、`uv_data[..u_plane_size]` / `uv_data[u_plane_size..]` を長さチェックなしで取る。`uv_data.len() < u_plane_size * 2` のときパニックする。

### 対象箇所

- `examples/player.rs` の `enqueue_video_frame` 内 I420 分岐

## 設計方針

長さ不足ならフレームをスキップする（または `Err` を返す）。パニックさせない。

## 完了条件

- UV 長不足でもパニックせず、警告またはスキップで継続できる
- 正当な I420 フレームは従来どおり再生できる

## 解決方法

1. `uv_data.len()` と必要バイト数を比較する
2. 不足時は早期 return（既存の Unknown と同様の扱いでもよい）
3. 必要なら一度だけの警告を出す
