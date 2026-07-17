# DOCS_RS ダミー bindings が不完全で cargo check が失敗する

- Priority: Medium
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-docs-rs-dummy-bindings-incomplete
- Polished: 2026-07-17

## 目的

`DOCS_RS=1` 時に出力するダミー `bindings.rs` を、クレートが参照する記号・型・関数署名に揃え、`cargo check` でも型検査が通るようにする。

（実行時の `SDL_VIDEODRIVER=dummy` とは別物。こちらは compile-time の bindings ダミーである。）

## 優先度根拠

- 公開 docs.rs / CI `docs-rs` ジョブ（`DOCS_RS=1 cargo doc --no-deps`）は現状常に成功する
- 一方 `DOCS_RS=1 cargo check` は約 62 件失敗する（実測例: E0425 約 58、E0609 約 3、E0277 1）
- CI は `cargo check` を走らせないため、ダミーと `src/` の乖離が緑のまま蓄積する
- ユーザー向けランタイムの即時障害ではないが、壊れた窓として放置すると docs.rs 経路の潜伏バグになるため Medium

## 現状

`build.rs` の `DOCS_RS` 分岐（おおむね L35–69）は、opaque 型・一部イベント定数・キー定数など最小限だけを書き出して return する。コメントも「ドキュメント生成時に最低限」と明記している。リンク指示は出さない。

一方 `src/` は `ffi::` 経由で SDL 関数・定数・`SDL_Event` フィールドを広く参照する。`src/ffi.rs` は `OUT_DIR` の `bindings.rs` を `include!` するだけなので、ダミー不足はそのまま名前解決・型エラーになる。

### 再現

```bash
DOCS_RS=1 cargo check          # 失敗（約 62 errors）
DOCS_RS=1 cargo doc --no-deps  # 成功（現状・CI と同条件）
```

キャッシュ混在を避ける場合は先に `cargo clean -p raw_player` する。

### なぜ doc は通って check は落ちるか

rustdoc は公開アイテムのシグネチャは検査するが関数本体は型検査しない。欠落した `ffi::SDL_Init` 等は `cargo check` だけが検出する。

### 不足の内訳（現状ダミーに無いもの）

必須集合は `src/` の `ffi::` 参照 **全体**（既存ダミーにある opaque・イベント定数・`SDLK_*`・`SDL_BLENDMODE_BLEND` 等も含む）。以下は **不足分のみ** のチェックリストであり、一括書き換え時に既存分を落とさないこと。

関数（ダミーに無し、呼び出しあり）:

- 初期化: `SDL_Init`, `SDL_Quit`, `SDL_GetError`
- Window: `SDL_CreateWindow`, `SDL_DestroyWindow`, `SDL_GetWindowSize`, `SDL_SetWindowSize`, `SDL_SetWindowTitle`
- Renderer: `SDL_CreateRenderer`, `SDL_DestroyRenderer`, `SDL_RenderClear`, `SDL_RenderPresent`, `SDL_RenderTexture`, `SDL_SetRenderDrawColor`, `SDL_SetRenderDrawBlendMode`, `SDL_RenderFillRect`, `SDL_RenderDebugText`, `SDL_GetRenderOutputSize`, `SDL_GetRenderScale`, `SDL_SetRenderScale`, `SDL_GetRendererName`, `SDL_SetRenderLogicalPresentation`, `SDL_SetRenderVSync`
- Texture: `SDL_CreateTexture`, `SDL_DestroyTexture`, `SDL_UpdateYUVTexture`, `SDL_UpdateNVTexture`, `SDL_UpdateTexture`
- Audio: `SDL_OpenAudioDeviceStream`, `SDL_DestroyAudioStream`, `SDL_PutAudioStreamData`, `SDL_GetAudioStreamQueued`, `SDL_PauseAudioStreamDevice`, `SDL_ResumeAudioStreamDevice`, `SDL_ClearAudioStream`, `SDL_SetAudioStreamGain`
- その他: `SDL_GetTicksNS`, `SDL_PollEvent`

定数（ダミーに無し、参照あり）:

- `SDL_INIT_VIDEO`, `SDL_INIT_AUDIO`
- `SDL_AudioFormat_SDL_AUDIO_S16`, `SDL_AudioFormat_SDL_AUDIO_F32`
- `SDL_PixelFormat_SDL_PIXELFORMAT_IYUV` / `NV12` / `YUY2` / `RGBA8888` / `ARGB8888`
- `SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING`
- `SDL_RendererLogicalPresentation_SDL_LOGICAL_PRESENTATION_LETTERBOX`

`SDL_Event`（現状: `type_` + `_pad: [u8; 124]` + `#[derive(Default)]`）:

- `src/event.rs` は `event.key.key`（keycode）と `event.window.data1` / `data2` を読む → E0609
- `#[derive(Default)]` + `[u8; 124]` は現行 rustc で確定 E0277（配列の `Default` は長さ 32 まで）

対象外:

- `src` 側の手動定数（`SDL_WINDOW_*`, `SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK` など）
- allowlist のみで `src` 未使用の `SDL_LockTexture` / `SDL_UnlockTexture`（allowlist 一括で入れるのは可、必須ではない）

### 対象ファイル

- 主変更: `build.rs` の `DOCS_RS` 分岐（コメント方針の更新を含む）
- 回帰防止（本バグの検知手段）: `.github/workflows/ci.yml` の `docs-rs` ジョブ
- 参照元（編集しない）: `src/` の `ffi::` 利用箇所。列挙は `rg 'ffi::' src` で再取得できる

## 設計方針

### AGENTS の「スタブ禁止」との関係

AGENTS / shiguredo-rust の「モックやスタブは絶対に利用しない」は **テスト用の偽物依存** を禁ずるもの。docs.rs はネットワーク／ネイティブビルド不可のため、`build.rs` が `DOCS_RS` 時だけ出す compile-time **ダミー** は既存の必須経路であり、本 issue はその網羅性を直す。本文でもこの経路は「ダミー」と呼ぶ。

### 同期の正本

- **必須集合**: `src/` が参照する関数・定数・型の全体（既存ダミー分＋下記不足分）
- **任意**: bindgen allowlist 関数をまとめてダミー化してもよい（その場合 Lock/Unlock を含めてよい）
- **定数**: `src` の `ffi::` 参照のみ。ワイルドカード allowlist の全定数はコピーしない
- ドリフト防止の必須手段は `DOCS_RS=1 cargo check`（CI でも実行）

### ダミー関数の書き方

- `#[link(...)]` を付けない。Rust の `pub fn` / `pub unsafe fn` でダミー戻り値（`false` / `null_mut()` / `0` / `()` 等）を返す形を推奨
- C 型は `ffi` に import が無いため、`::std::os::raw::c_char` / `c_void` など完全修飾を使う
- `extern "C" { fn ...; }` でも lib の `cargo check` は通りやすいが、リンク前提の宣言を実 bindings から無批判にコピーしない
- DOCS_RS 分岐は引き続きリンク指示を出さず return する（ジョブに apt / libclang が無い）

### ダミーの署名

名前だけでは足りない。呼び出しが型検査に使う戻り値・引数を合わせる。細部は `DOCS_RS=1 cargo check` の型エラーを潰してよい。

| 戻り値 | 例 |
|--------|-----|
| `bool` | `SDL_Init`, `SDL_PollEvent`, `SDL_RenderClear`, `SDL_PutAudioStreamData` など |
| `*mut T` | `SDL_CreateWindow` / `CreateRenderer` / `CreateTexture`, `SDL_OpenAudioDeviceStream` |
| `*const c_char` | `SDL_GetError`, `SDL_GetRendererName` |
| `u64`（固定） | `SDL_GetTicksNS`（`u64` への代入・減算がある） |
| `i32` | `SDL_GetAudioStreamQueued` |
| `()` | `SDL_Quit`, 各 `Destroy*` |

引数・関連型の注意:

- `SDL_CreateWindow` の `flags` は `u64` 相当
- `SDL_OpenAudioDeviceStream`: `*const SDL_AudioSpec`, コールバックは `Option<...>`（`None`）, userdata は `*mut c_void`（`null_mut()`）
- `SDL_PutAudioStreamData` の data は `*const c_void` 相当（`as_ptr().cast()`）
- `SDL_Update*Texture` / `SDL_RenderTexture` の rect は `null()` を渡す。実署名を写すなら `SDL_Rect` / `SDL_FRect` が要る。`*const ()` 等で `null()` が通る形でも可
- `SDL_AudioFormat` typealias と `SDL_AudioSpec.format`、および `SDL_AudioFormat_SDL_AUDIO_*` 定数の型はダミー内で一致させる（実 SDL の数値一致は不要）

### `SDL_Event` ダミーの最小形

本物の全 union は不要。`event.rs` が要求する最小限:

- `type_: u32`（または同等）
- `key`: ネスト型。公開フィールド **`key: u32`**（`event.key.key` が keycode）
- `window`: ネスト型。公開フィールド **`data1: i32`**, **`data2: i32`**
- `Default` は付けない（巨大 `_pad` + derive は使わない）

### opaque 型と 0023

`Window` / `Renderer` / `Texture` は `NonNull<ffi::SDL_*>` を保持する。closed 0023 の結論どおり、`!Send` / `!Sync` の実効因は `NonNull` であり、FFI opaque の形ではない。本 issue では:

- `src/` のラッパは触らない（非目標）
- ダミーは既存どおり名前解決できる opaque（`pub struct SDL_Window;` 等の ZST）を維持し、所有データ化しない

### 非目標

- DOCS_RS 下での実 SDL リンク・実行・テスト
- 実 SDL とバイナリ互換のメモリレイアウト
- `pub mod sys` を完全な SDL API リファレンスに見せること（ダミー関数追加で docs.rs 上の `sys` 公開面が増えるのは許容）
- `src/` のランタイム実装変更

## 完了条件

- `DOCS_RS=1 cargo check` が成功する（本 issue の主ゲート。現状未達）
- `DOCS_RS=1 cargo doc --no-deps` が成功する（現状成功の回帰確認）
- `DOCS_RS` 未設定の通常 `cargo check`（および既存テスト経路）が壊れていない
- `.github/workflows/ci.yml` の `docs-rs` ジョブ（既に `env: DOCS_RS: 1`）に `cargo check` ステップを追加する。挿入位置は `rustup update stable` の直後、`cargo doc --no-deps` の直前（fail-fast）。本バグの回帰固定であり別スコープではない
- `build.rs` の DOCS_RS コメントを「最低限」から「`src` の参照と揃え `cargo check` が通る」方針に更新する（allowlist 一括は任意のまま。修正後に旧コメントが虚偽になるための事実更新）
- `src/` のラッパを変えず、0023 が確認した `NonNull` 由来の `!Send` / `!Sync` を壊さない

## 解決方法

1. `rg 'ffi::' src` と上記不足リスト、必要なら実 bindings の署名を突き合わせ、既存ダミーを落とさず不足分（関数・定数・`SDL_Event` 最小形）を追加する（`#[link]` なし）
2. 戻り値・主要引数を呼び出しに合わせる。残りは `DOCS_RS=1 cargo check` の型エラーを潰す
3. `SDL_Event` を設計方針の最小形にする（`Default` / 巨大 `_pad` をやめる）
4. 完了条件どおり、ローカル確認・CI `docs-rs` への `cargo check` 追加・`build.rs` コメント更新を行う

### テスト方針

- PBT / 単体テストは対象外（docs.rs 用ダミーの compile-time 網羅であり、実行時プロパティでも境界値ユニットでもない）
- 検証は完了条件のコマンドと CI で行う
