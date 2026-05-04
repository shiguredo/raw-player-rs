---
name: raw-player
description: raw_player クレート (Rust) を使ってデコード済みの音声/映像フレームを PTS ベースで AV 同期再生する。PCM (S16/F32) と I420/NV12/YUY2/RGBA/BGRA を SDL3 でレンダリングする際に使用する。VideoPlayer / AudioPlayer / 低レベル API (Window, Renderer, Texture, Event)、source-build feature、統計オーバーレイ、VSync 設定を扱うときに使う。
---

# raw-player

このスキルは `raw_player` クレートを Rust コードに組み込むときに使う。デコード済みフレームを受け取り、PTS (Presentation Timestamp) に基づいて音声と映像を同期しながら再生するライブラリ。

## クレートが提供するもの

- `VideoPlayer` — ウィンドウ作成、フレーム描画、AV 同期、統計オーバーレイ
- `AudioPlayer` — PCM キューイング、再生クロック取得 (音声のみ用途)
- `Window` / `Renderer` / `Texture` — SDL3 ベースの低レベル API
- `Event` / `poll_event` / `init` / `quit` — イベントループ用関数群
- `AudioFormat` — `S16` / `F32` の PCM フォーマット指定

対応音声: PCM (S16, F32)
対応映像: I420 (YUV420P), NV12, YUY2, RGBA, BGRA

## API の選び方

1. AV 同期再生をしたい → `VideoPlayer` 一つで音声と映像の両方を扱う。音声 PTS をマスタークロックとして映像が同期される
2. 音声のみ再生したい → `AudioPlayer` を使う。`raw_player::init()` を先に呼ぶ
3. ウィンドウや描画を直接制御したい → `Window` / `Renderer` / `Texture` を使う。終了時は `drop` してから `unsafe { quit() }` を呼ぶ

`VideoPlayer` を使う場合、`enqueue_audio` を呼ばなければ映像のみ再生になる。映像のみ再生では映像 PTS がそのまま再生時刻になる。

## ビルド

デフォルトでは GitHub Releases から prebuilt SDL3 バイナリをダウンロードしてビルドする。

```bash
cargo build
```

SDL3 をソースからビルドする場合は `source-build` feature を有効にする。CI とローカル開発でこれを使うことが多い。

```bash
cargo build --features source-build
```

Linux でソースビルドする際は `libclang-dev`, `libasound2-dev`, `libpulse-dev`, `libx11-dev`, `libxext-dev`, `libxfixes-dev`, `libxrandr-dev`, `libxi-dev` が必要。

## 映像フォーマット別のキューイング

PTS の単位は全 API でマイクロ秒 (`u64`)。

### I420 (YUV420P)

3 プレーン (Y / U / V) を別々に渡す。U と V は幅・高さがともに半分。

```rust
let y = vec![0u8; 1920 * 1080];
let u = vec![0u8; 960 * 540];
let v = vec![0u8; 960 * 540];
player.enqueue_video_i420(&y, &u, &v, 1920, 1080, pts_us)?;
```

### NV12

Y プレーンと UV インターリーブプレーンを渡す。UV プレーンは幅 = Y 幅、高さ = Y 高さ / 2。

```rust
let y = vec![0u8; 1920 * 1080];
let uv = vec![0u8; 1920 * 540];
player.enqueue_video_nv12(&y, &uv, 1920, 1080, pts_us)?;
```

### YUY2

パックドフォーマット (2 ピクセル = 4 バイト)。

```rust
let data = vec![0u8; 1920 * 1080 * 2];
player.enqueue_video_yuy2(&data, 1920, 1080, pts_us)?;
```

### RGBA / BGRA

1 ピクセル = 4 バイト。

```rust
player.enqueue_video_rgba(&data, 1920, 1080, pts_us)?;
player.enqueue_video_bgra(&data, 1920, 1080, pts_us)?;
```

`enqueue_video_bgra_owned` は `Vec<u8>` を所有移譲してゼロコピーで渡す。コピーコストを抑えたいときに使う。

## AV 同期再生

音声 PTS がマスタークロックになる。音声と映像の両方を同じ `VideoPlayer` に enqueue し、`poll_events()` を回すだけで自動同期される。

```rust
use raw_player::{VideoPlayer, AudioFormat};

let player = VideoPlayer::new(1920, 1080, "AV Sync Player")?;

player.enqueue_video_i420(&y, &u, &v, 1920, 1080, video_pts_us)?;
player.enqueue_audio(&audio_data, audio_pts_us, 48000, 2, AudioFormat::F32)?;

player.play()?;

loop {
    if !player.poll_events()? {
        break;
    }
}

player.close();
```

`poll_events()` は閉じられたら `false` を返す。これでメインループを終了する。

## 音声のみ再生

```rust
use raw_player::{AudioPlayer, AudioFormat};

raw_player::init()?;

let player = AudioPlayer::new();
player.enqueue_audio(&pcm_data, 0, 48000, 2, AudioFormat::F32)?;
player.play()?;
```

`AudioPlayer` を使う場合は明示的に `raw_player::init()` を先に呼ぶ必要がある。`VideoPlayer::new()` は内部で `init()` を呼ぶので不要。

## 低レベル API

ウィンドウ、レンダラー、テクスチャを直接操作する場合の典型的な流れ。

```rust
use raw_player::{init, quit, poll_event, Event, Window, Renderer};

fn main() -> raw_player::Result<()> {
    init()?;

    let window = Window::new("Example", 640, 480)?;
    let renderer = Renderer::new(&window)?;

    loop {
        while let Some(event) = poll_event() {
            if matches!(event, Event::Quit) {
                drop(renderer);
                drop(window);
                unsafe { quit() };
                return Ok(());
            }
        }
    }
}
```

`quit()` は `unsafe`。SDL リソース (`Window`, `Renderer`, `Texture`) を全部 `drop` してから呼ぶ必要がある。順序を間違えると未定義動作になる。

## VideoPlayer の運用 API

頻出するもの:

- `play()` / `pause()` / `stop()` — `stop()` はキューをクリアする
- `close()` — ウィンドウを閉じる
- `drain_video()` — 映像キューだけクリアし、タイミング状態をリセット (シーク等)
- `set_max_video_queue_size(size)` — キュー溢れ対策
- `set_vsync(interval)` — 0 で無効、1 で有効
- `set_stats_overlay(enabled)` — 画面に FPS や同期差を重ね描き
- `set_key_callback(callback)` — キーイベントのフック
- `stats()` — `VideoPlayerStats` を取得 (フレーム数、ドロップ数、PTS 差分、レンダリング時間など)
- `set_volume(0.0..=1.0)` — 音量

## サンプル: examples/player

`shiguredo_video_device` (カメラ) と `shiguredo_audio_device` (マイク) で取得した生データをそのまま `VideoPlayer` に流し込むサンプル。`source-build` を使うのが現実的。

```bash
cargo run --example player -- --list-devices
cargo run --example player
cargo run --example player -- --resolution 1080p --fps 60
cargo run --example player -- --resolution 1920x1080
cargo run --example player -- --video-input-device <id> --audio-input-device <id>
cargo run --example player -- --duration 30
```

実行中の操作: ESC で終了、S で統計オーバーレイ切替。

## ハマりやすい点

- PTS 単位はマイクロ秒。ミリ秒や秒と混同しない
- I420 の U/V プレーンサイズは Y の 1/4 (幅も高さも半分)。バッファ長を間違えるとパニックする
- NV12 の UV プレーンは幅 = Y 幅 (インターリーブのため)、高さ = Y 高さ / 2
- 低レベル API では `Window` / `Renderer` / `Texture` を `quit()` より先に `drop` する
- `AudioPlayer` 単独利用時は `raw_player::init()` を忘れない
- ソースビルドは Linux で apt パッケージが必要。CI の `.github/workflows/ci.yml` を参照
- 対応プラットフォーム: macOS 26/15 (arm64), Ubuntu 24.04/22.04 (x86_64/arm64), Windows Server 2025 / Windows 11 (x86_64)

## テスト構成

- 単体テスト: `tests/test_<module>.rs` または `src/<module>.rs` の `#[cfg(test)]`
- PBT (Property-Based Testing): `pbt/tests/prop_<module>.rs` で proptest を使う
- Fuzzing: `cargo-fuzz`、`make fuzzing` / `make fuzzing-parallel`
- カバレッジ: `make cover` (`cargo llvm-cov`)
