# pause/stop/play の SDL 失敗時にフラグとデバイス状態が食い違う

- Priority: High
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-pause-stop-partial-failure-state
- Polished: 2026-07-22

## 目的

`AudioPlayer` / `VideoPlayer` の `pause` / `stop` / `play` で SDL 操作が失敗したとき、内部フラグとデバイス状態が食い違わないようにする。

## 優先度根拠

部分失敗後は「停止扱いなのに音声が鳴り続ける」「映像は playing のまま音声だけ止まった／音声だけ再生中」など制御不能な状態になりうる。さらに現行実装では失敗後の再 `pause` / `play` が no-op の `Ok(())` になり、回復経路が無い。AV プレイヤーの状態機械として許容できない。

## 現状

### AudioPlayer（`src/audio_player.rs`）

| API | 現行の危険な順序 | 失敗時の結果 |
| --- | --- | --- |
| `pause`（140–148 行付近） | `playing = false` → `stream.pause()?` | フラグだけ停止。`playing == false` のため再 `pause()` は本体スキップで `Ok(())`。デバイスは再生したまま回復不能 |
| `play`（123–136 行付近） | `playing`/`has_played = true` → `process_audio_queue?` → `resume?` | フラグだけ再生中。再 `play()` も no-op `Ok(())` |
| `stop`（152–167 行付近） | フラグ更新 + **`audio_queue.clear()`（不可逆）** → `pause?`/`clear?` → カウンタリセット | キュー喪失 + カウンタ未リセット + デバイス未停止が同時に起きうる |

`stream` が `None` のときはデバイス操作なしでフラグだけ変わる（その経路では本バグの SDL 部分失敗は起きない）。

### VideoPlayer（`src/video_player.rs`）

`play` / `pause` / `stop`（698–741 行付近）は映像側で fallible な SDL を呼ばない。やることは `self.audio.*()?` のあと映像 `inner` のフラグ等を更新することだけ。

したがって「audio 成功・映像 SDL 失敗」は現行 API では起きない。食い違いは **`AudioPlayer` 内部の部分失敗** が `Err`（または意味の壊れた状態）を返したとき、映像側フラグが更新されない／audio 側だけ進むことで起きる。

例: `AudioPlayer::pause` がフラグだけ落として `Err` → `VideoPlayer::pause` は映像 `playing` を更新しない → 映像は再生中のまま、音声フラグは停止。

### 対象箇所

- `src/audio_player.rs` の `play` / `pause` / `stop`（主）
- `src/video_player.rs` の `play` / `pause` / `stop`（audio 成功後のみ映像更新を維持・明文化）
- rustdoc（失敗時契約）
- 退行: 成功経路の単体 + コード構造による保証（下記）

`src/audio_stream.rs` の API シグネチャ変更は不要（読取り参照のみ）。

### 関連

- open 0016: 同じ `VideoPlayer::play`/`pause` に `pause_started_ns` 補正を入れる。目的はクロック。**推奨実装順は 0016 → 0018**。0018 は「`audio.?` 成功後にだけ映像更新」を壊さないこと。0016 は本 issue の原子性には踏み込まない
- closed 0001: `stop` の `has_played` リセット（解決済み）。本 issue は SDL 失敗時の一貫性で別問題
- closed 0004: pause 中 `NotPlaying`。フラグだけ停止だと音が残り契約が空文化しうる
- closed 0007: `put_data` 失敗時のチャンク `push_front`。一時退避の先例だが、`stop` のキュー全破棄や `resume` 失敗後のストリーム内バッファには使えない

### 本 issue の範囲外

- `AudioPlayer::set_volume`（`volume` 先行 + `set_gain?`）。同型の部分失敗だが pause/stop/play とは別。必要なら別 issue
- open 0024 の横断カバレッジ（本 issue は状態一貫性の最小セット）

## 設計方針

**採用: デバイス操作の成功後にだけローカル状態を更新する。補償トランザクション（失敗後の逆操作）は採用しない。**

不可逆操作（`audio_queue.clear()` 等）を SDL より前に置かない。

### AudioPlayer の期待順序

| API | 期待順序 | `Err` 時に維持すべき観測 |
| --- | --- | --- |
| `pause` | `stream.pause()?`（あれば）→ 成功後に `playing = false` | `is_playing` と実デバイスが一致。失敗時は呼び出し前の論理状態を維持し、再試行で本体が再実行される |
| `play` | `process_audio_queue?` → `resume?`（あれば）→ 成功後に `playing`/`has_played`/`play_start_time_ns` を確定 | 失敗時は呼び出し前のフラグを維持（入口で `has_played` を戻さない／途中で立てない）。`process` 成功・`resume` 失敗時: ストリームに載った PCM と `audio_started` 等の副作用は残す（ロールバックしない）。再 `play` では app キューは既に空のため二重 `put_data` はせず `resume` 再実行が本体。初回失敗なら `has_played==false` のまま enqueue 可。pause 後の再 `play` 失敗なら `has_played && !playing` 維持で `NotPlaying` 契約は壊れない |
| `stop` | `stream.pause()?` → `stream.clear()?`（あれば）→ 成功後にフラグ・キュー破棄・カウンタリセット | `pause` 成功・`clear` 失敗: デバイスは pause 済み・アプリキューは未クリア・フラグは呼び出し前のまま（通常 `playing==true`）。その間 `process()` が追記しうる。再 `stop` は `pause`→`clear`→ローカル破棄をそのまま再実行してよい（既 pause への再 `pause` は冪等として許容）。再試行の `clear` は追記分も含めて破棄する |

`stream == None` のときはデバイス操作をスキップし、ローカル状態だけを従来どおり更新してよい。

### VideoPlayer

「audio/video 同一トランザクション」という曖昧語は使わない。契約は次に固定する。

- 映像側に pause/stop/play 用の SDL 失敗点は無い
- 一貫性の本体は `AudioPlayer` の all-or-nothing
- `audio.*()` が `Ok` のときだけ映像フラグ（および 0016 の `pause_started_ns` 等）を更新する。audio `Err` なら映像側は一切触らない（現行構造を維持）

## 完了条件

- SDL 操作失敗時に「フラグだけ更新」「キューだけ破棄」「片方のプレイヤーだけ更新」が残らない
- 失敗後に同じ API を再試行すると、デバイス操作が再度実行される（現行の no-op `Ok` デッドエンドを解消）
- 失敗時の契約（`Err` の意味・再試行可否・`stop` の `pause` Ok/`clear` Err 例外）が rustdoc で分かる
- 下記の検証がある

### テスト戦略

モック禁止のため SDL 失敗の安定注入はしない。保証は次の二層。

1. **構造保証（必須）**
   レビュー観点: `playing` / `has_played` / キュー `clear` / カウンタリセットが、その関数内のデバイス操作の `?` より前に無いこと（`stream == None` のローカルのみ更新は除く）。並べ替え後の `play` は入口で `playing==false` のため、`process_audio_queue` 内の条件付き `resume` に頼らず、外側の `resume?` が必須
2. **成功経路の単体（必須）**
   - `tests/test_audio_player.rs`（新規）: `init` →（必要なら enqueue）→ play → `is_playing`、pause → 非 playing、stop 後に再 enqueue して再 play 可能
   - 環境: `SDL_AUDIODRIVER=dummy`（必要なら）
   - 失敗原子性はここでは証明しないと明記。広いカバレッジは 0024

## 解決方法

1. `AudioPlayer::pause` / `play` / `stop` を上記の期待順序に並べ替える（不可逆クリアを SDL 成功後へ）
2. 失敗後も再試行で本体が走るよう、フラグ先行更新をやめる
3. `VideoPlayer` は audio 成功後のみ映像更新の構造を維持し、rustdoc で契約を書く
4. `AudioPlayer` / `VideoPlayer` の `play`/`pause`/`stop` に失敗時契約の rustdoc を追加する
5. 成功経路の単体テストを追加する
