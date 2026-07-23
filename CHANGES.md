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

- [UPDATE] SDL 3.4.12 にアップデートする
  - @voluntas
- [UPDATE] shiguredo_cmake を 4.4 に、shiguredo_toml を 2026.2 にアップデートする
  - @voluntas
- [UPDATE] 開発用依存の shiguredo_audio_device / shiguredo_video_device を 2026.2.0-canary にアップデートする
  - @voluntas
- [FIX] video_player.rs の手動ゼロ除算チェックを checked_div に置き換える
  - @voluntas
- [FIX] 映像のみ再生で pause 中の壁時計が進み再開直後にフレームが同期ドロップする問題を修正する
  - @voluntas

### misc

- [UPDATE] prek.toml の builtin hooks を拡充し end-of-file-fixer の rustfmt 競合を回避する
  - @voluntas
- [ADD] skills/raw-player/SKILL.md を追加する
  - @voluntas

## 2026.1.0

**リリース日**: 2026-04-03
