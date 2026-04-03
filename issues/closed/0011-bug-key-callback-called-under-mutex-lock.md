# key_callback が Mutex ロック保持中に呼ばれる

Created: 2026-04-03
Completed: 2026-04-03
Model: Opus 4.6

## 概要

`poll_events()` は `inner` の Mutex を保持したまま `key_callback` を呼んでいる (video_player.rs:750)。コールバック内で `VideoPlayer` の他のメソッドを呼ぶとデッドロックし、コールバックが panic すると Mutex が poison されてその後の全操作が panic 連鎖する。

## 根拠

- コールバックはユーザー提供コードであり、何をするか予測できない
- Mutex ロック中にユーザーコードを呼ぶのは Rust のベストプラクティスに反する
- poison 後の `lock().unwrap()` で全公開 API が連鎖的に panic する

## 対応方針

キーイベント処理でコールバックを呼ぶ前にロックを解放する。callback の結果を受けて再度ロックを取得し、状態を更新する。

## 解決方法

`poll_events()` のキーイベント処理を 2 段階に分離した。S キーのトグル処理はロック内で行い、コールバック呼び出しはロックを解放してから実行する。`key_callback` の型を `Box<dyn Fn>` から `Arc<dyn Fn>` に変更し、ロック外で安全にクローンして呼べるようにした。`set_key_callback` の型制約に `Sync` を追加した。
