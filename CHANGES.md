# 変更履歴

- UPDATE
  - 後方互換がある変更
- ADD
  - 後方互換がある追加
- CHANGE
  - 後方互換のない変更
- FIX
  - バグ修正

## develop

- [CHANGE] `PixelBufferLock::stride()` の戻り値を `Result<i32>` に変更して `i32` 範囲外の stride を安全に検出する
  - @voluntas
- [FIX] `PixelBufferLock::stride()` が `usize` を `as i32` で無検証に切り詰めていた問題を修正する
  - @voluntas
- [FIX] サンプル `player.rs` の `expect()` を `eprintln!` + `process::exit(1)` に置き換えて recoverable error での panic を防止する
  - @voluntas
- [FIX] `enqueue_video_pixel_buffer()` に `MAX_DIMENSION` チェックを追加し、レンダリング時に CVPixelBuffer の実プレーンサイズと宣言サイズの整合を検証する
  - @voluntas
- [CHANGE] `quit()` を `unsafe fn` に変更して SDL リソース解放前の呼び出しを型レベルで警告する
  - @voluntas
- [CHANGE] `set_key_callback` のクロージャ制約に `Sync` を追加する
  - @voluntas
- [FIX] `poll_events()` の `key_callback` を Mutex ロック外で呼び出しデッドロックと poison 連鎖を防止する
  - @voluntas
- [FIX] validate 関数に `MAX_DIMENSION` 上限チェックを追加し pitch 計算の i32 オーバーフローを防止する
  - @voluntas
- [ADD] prebuilt バイナリのダウンロードに対応する
  - @voluntas
- [ADD] `BUILD_REPOSITORY` / `BUILD_VERSION` 定数を公開する
  - @voluntas
- [ADD] docs.rs ビルドに対応する
  - @voluntas
- [CHANGE] 再生停止中にデータを投入した場合に `Error::NotPlaying` エラーを返すようにする
  - @voluntas
- [CHANGE] `Window` と `Renderer` を `Send` にしないよう型で表現し、`VideoPlayer` のスレッド境界越し利用を型で拒否する
  - @voluntas
- [FIX] `AudioPlayer::stop()` が `has_played` をリセットしない問題を修正する
  - @voluntas
- [FIX] 音声フォーマット変更時にクロック基準 (`first_pts_us`) が更新されない問題を修正する
  - @voluntas
- [FIX] `PixelBufferLock::plane` でプレーン先頭ポインタのヌルと `stride * height` のオーバーフローを検証する
  - @voluntas
- [FIX] `AudioStream` への書き込みやストリーム開設が失敗したときに dequeue した音声チャンクをキューへ戻す
  - @voluntas
- [FIX] `init()` / `quit()` で SDL 初期化状態を `Mutex` で守り、並行初回 `init` で `SDL_Init` 完了前に成功を返さないようにする
  - @voluntas

### misc

- `plane_buffer_len` の単体テスト、Linux 向け並行 `init` テスト、`process_audio_queue` のキュー復帰と同じ順序のモデルテストを追加する
  - @voluntas

