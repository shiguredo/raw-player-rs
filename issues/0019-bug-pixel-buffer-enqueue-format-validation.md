# enqueue_video_pixel_buffer が対応フォーマットと偶数寸法を enqueue 時に検証しない

- Priority: High
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-pixel-buffer-enqueue-format-validation
- Polished:

## 目的

`enqueue_video_pixel_buffer` が I420/NV12 以外や奇数寸法を受け付け、描画時まで失敗を遅延させないようにする。

## 優先度根拠

他の enqueue API は投入時点で厳密検証する。PixelBuffer パスだけが緩く、描画時失敗では既にキューから pop 済みでフレームが失われる。過去の寸法検証強化後も、フォーマットと偶数制約の前倒しが残っている。

## 現状

`enqueue_video_pixel_buffer` は width/height の正値と `MAX_DIMENSION` のみ検証する。偶数寸法チェックや I420/NV12 以外の拒否がない。

描画時 (`render_frame_internal`) で非対応フォーマットは `Err` になるが、その時点で `pop_front` 済み。

### 対象箇所

- `src/video_player.rs` の `enqueue_video_pixel_buffer`
- `src/video_player.rs` の PixelBuffer 描画分岐（失敗時の扱い）

## 設計方針

enqueue 時点で `VideoFormat` が I420/NV12 であること、およびフォーマットに応じた偶数寸法を検証する。描画失敗時のフレーム喪失を減らす。

## 完了条件

- 非対応フォーマットや奇数寸法は enqueue が `Err` を返し、キューに入らない
- I420/NV12 の正当な入力は従来どおり enqueue できる
- エラーパスのテストがある

## 解決方法

1. enqueue 時に対応フォーマットと偶数寸法を検証する
2. 必要なら pitch の下限も他 API に合わせて検証する
3. 単体テストを追加する
