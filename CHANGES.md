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

- [CHANGE] 再生停止中にデータを投入した場合に `Error::NotPlaying` エラーを返すようにする
  - @voluntas
- [FIX] `AudioPlayer::stop()` が `has_played` をリセットしない問題を修正する
  - @voluntas
- [FIX] 音声フォーマット変更時にクロック基準 (`first_pts_us`) が更新されない問題を修正する
  - @voluntas

### misc

