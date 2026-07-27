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


## 2026.2.0

**リリース日**: 2026-07-27

- [UPDATE] SDL 3.4.12 にアップデートする
  - @voluntas
- [UPDATE] shiguredo_cmake を 4.4 に、shiguredo_toml を 2026.2 にアップデートする
  - @voluntas
- [UPDATE] 開発用依存の shiguredo_audio_device / shiguredo_video_device を 2026.2 にアップデートする
  - @voluntas
- [FIX] video_player.rs の手動ゼロ除算チェックを checked_div に置き換える
  - @voluntas
- [FIX] 映像のみ再生で pause 中の壁時計が進み再開直後にフレームが同期ドロップする問題を修正する
  - @voluntas
- [FIX] 公開 Texture::update_* がバッファ長未検証のまま FFI する問題を修正する
  - @voluntas
- [FIX] pause/stop/play の SDL 失敗時にフラグとデバイス状態が食い違う問題を修正する
  - @voluntas
- [FIX] enqueue_video_pixel_buffer が対応フォーマットと偶数寸法を enqueue 時に検証しない問題を修正する
  - @voluntas
- [FIX] I420 PixelBuffer 描画で V プレーン stride を見ない問題を修正する
  - @voluntas
- [FIX] examples/player の I420 UV 分割が短バッファでパニックする問題を修正する
  - @voluntas
- [FIX] DOCS_RS ダミー bindings が不完全で cargo check が失敗する問題を修正する
  - @voluntas
- [FIX] Texture::new が非正・過大の寸法を FFI 前に拒否しない問題を修正する
  - @voluntas

### misc

- [UPDATE] prek.toml の builtin hooks を拡充し end-of-file-fixer の rustfmt 競合を回避する
  - @voluntas
- [ADD] skills/raw-player/SKILL.md を追加する
  - @voluntas
- [FIX] SDL を使う統合テストをプロセス内 Mutex で直列化し並列実行時のクラッシュを防ぐ
  - @voluntas

## 2026.1.0

**リリース日**: 2026-04-03
