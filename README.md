# raw-player-rs

[![raw_player](https://img.shields.io/crates/v/raw_player.svg)](https://crates.io/crates/raw_player)
[![Documentation](https://docs.rs/raw_player/badge.svg)](https://docs.rs/raw_player)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

## About Shiguredo's open source software

We will not respond to PRs or issues that have not been discussed on Discord. Also, Discord is only available in Japanese.

Please read <https://github.com/shiguredo/oss/blob/master/README.en.md> before use.

## 時雨堂のオープンソースソフトウェアについて

利用前に <https://github.com/shiguredo/oss> をお読みください。

## raw-player-rs について

デコード済みフレームを受け取り、AV 同期しながら描画する Rust ライブラリです。

PCM / I420 / NV12 / YUY2 / RGBA / BGRA データを PTS (Presentation Timestamp) に基づいて音声と映像を同期しながら再生します。

<https://github.com/user-attachments/assets/6f0bea25-f38d-4e3d-bc4d-885d475c20d4>

## Python 版

[shiguredo/raw\-player: Raw audio/video player for Python](https://github.com/shiguredo/raw-player)

## 特徴

- 生の音声/映像入力データをそのまま再生できる
- 音声フォーマットは PCM (S16 / F32) に対応
- 映像フォーマットは I420 (YUV420P) / NV12 / YUY2 / RGBA / BGRA に対応
- PTS ベース音声をマスタークロックとした映像同期機能
- GPU レンダリング (SDL3)
- 統計オーバーレイ表示
- prebuilt バイナリによる高速ビルド (デフォルト)
- ソースからのビルドも可能 (`--features source-build`)

## 対応プラットフォーム

- macOS 26 arm64
- macOS 15 arm64
- Ubuntu 24.04 x86_64
- Ubuntu 24.04 arm64
- Ubuntu 22.04 x86_64
- Ubuntu 22.04 arm64
- Windows Server 2025 x86_64
- Windows 11 x86_64

## ビルド

デフォルトでは GitHub Releases から prebuilt バイナリをダウンロードしてビルドします。

```bash
cargo build
```

### ソースからビルド

SDL3 をソースからビルドする場合は `source-build` feature を有効にしてください。

```bash
cargo build --features source-build
```

## 機能

- `VideoPlayer` - ウィンドウ作成、フレーム描画、AV 同期再生、統計オーバーレイ
- `AudioPlayer` - PCM キューイング、再生クロック取得
- `Window` - ウィンドウの作成と管理
- `Renderer` - レンダリングコンテキスト
- `Texture` - YUV テクスチャの作成と更新
- `Event` - イベントポーリング (キー入力、ウィンドウイベント、終了イベント)

## 使い方

### I420 (YUV420P) 再生

```rust
use raw_player::VideoPlayer;

fn main() -> raw_player::Result<()> {
    let player = VideoPlayer::new(1920, 1080, "I420 Player")?;

    // Y, U, V プレーンを用意
    let y_plane = vec![0u8; 1920 * 1080];
    let u_plane = vec![0u8; 960 * 540];
    let v_plane = vec![0u8; 960 * 540];

    // PTS (マイクロ秒) を指定してキューに追加
    player.enqueue_video_i420(&y_plane, &u_plane, &v_plane, 1920, 1080, 0)?;
    player.play()?;

    loop {
        match player.poll_events()? {
            true => {}
            false => break,
        }
    }

    player.close();
    Ok(())
}
```

### NV12 再生

```rust
let player = VideoPlayer::new(1920, 1080, "NV12 Player")?;

// Y プレーンと UV インターリーブプレーンを用意
let y_plane = vec![0u8; 1920 * 1080];
let uv_plane = vec![0u8; 1920 * 540];

player.enqueue_video_nv12(&y_plane, &uv_plane, 1920, 1080, 0)?;
```

### YUY2 再生

```rust
let player = VideoPlayer::new(1920, 1080, "YUY2 Player")?;

// YUY2: パックドフォーマット (2 ピクセルで 4 バイト)
let yuy2_data = vec![0u8; 1920 * 1080 * 2];

player.enqueue_video_yuy2(&yuy2_data, 1920, 1080, 0)?;
```

### RGBA / BGRA 再生

```rust
let player = VideoPlayer::new(1920, 1080, "RGBA Player")?;

let rgba_data = vec![0u8; 1920 * 1080 * 4];

player.enqueue_video_rgba(&rgba_data, 1920, 1080, 0)?;
// または
player.enqueue_video_bgra(&rgba_data, 1920, 1080, 0)?;
```

### PTS ベースの AV 同期再生

```rust
use raw_player::{VideoPlayer, AudioFormat};

let player = VideoPlayer::new(1920, 1080, "AV Sync Player")?;

// 映像フレームをキューに追加 (I420 形式)
player.enqueue_video_i420(&y, &u, &v, 1920, 1080, 0)?;

// 音声データをキューに追加
// data: S16 または F32 の PCM バイト列
player.enqueue_audio(&audio_data, 0, 48000, 2, AudioFormat::F32)?;

player.play()?;

loop {
    match player.poll_events()? {
        true => {}
        false => break,
    }
    // poll_events() が音声 PTS に基づいて適切なフレームを自動描画
}

player.close();
```

### 音声のみ再生

```rust
use raw_player::{AudioPlayer, AudioFormat};

raw_player::init()?;

let player = AudioPlayer::new();

// PCM データをキューに追加
player.enqueue_audio(&pcm_data, 0, 48000, 2, AudioFormat::F32)?;
player.play()?;
```

## API リファレンス

### VideoPlayer

映像再生用。音声も統合可能。

```rust
let player = VideoPlayer::new(width, height, title)?;
```

| メソッド | 説明 |
| --- | --- |
| `enqueue_video_i420(y, u, v, width, height, pts_us)` | I420 フレームをキューに追加 |
| `enqueue_video_nv12(y, uv, width, height, pts_us)` | NV12 フレームをキューに追加 |
| `enqueue_video_yuy2(data, width, height, pts_us)` | YUY2 フレームをキューに追加 |
| `enqueue_video_rgba(data, width, height, pts_us)` | RGBA フレームをキューに追加 |
| `enqueue_video_bgra(data, width, height, pts_us)` | BGRA フレームをキューに追加 |
| `enqueue_video_bgra_owned(data, width, height, pts_us)` | BGRA フレームをキューに追加 (ゼロコピー) |
| `enqueue_audio(data, pts_us, sample_rate, channels, format)` | 音声データをキューに追加 |
| `play()` | 再生開始 |
| `pause()` | 一時停止 |
| `stop()` | 停止してキューをクリア |
| `close()` | ウィンドウを閉じる |
| `poll_events()` | イベント処理とフレーム描画 (閉じられたら `false`) |
| `set_key_callback(callback)` | キーイベントコールバックを設定 |
| `stats()` | 統計情報を取得 |
| `drain_video()` | 映像キューをクリアしタイミングをリセット |
| `set_stats_overlay(enabled)` | 統計オーバーレイの表示/非表示 |
| `set_vsync(interval)` | VSync を設定 (0: 無効, 1: 有効) |
| `set_max_video_queue_size(size)` | 映像キューの最大サイズを設定 |
| `max_video_queue_size()` | 映像キューの最大サイズを取得 |

| プロパティ | 説明 |
| --- | --- |
| `is_open()` | ウィンドウが開いているか |
| `is_playing()` | 再生中か |
| `width()` | ウィンドウ幅 |
| `height()` | ウィンドウ高さ |
| `title()` | ウィンドウタイトル |
| `set_title(title)` | ウィンドウタイトルを設定 |
| `renderer_name()` | GPU レンダラー名 |
| `stats_overlay()` | 統計オーバーレイの表示状態 |
| `volume()` | 音量 (0.0 - 1.0) |
| `set_volume(volume)` | 音量を設定 |

### AudioPlayer

独立した音声再生用。

```rust
let player = AudioPlayer::new();
```

| メソッド | 説明 |
| --- | --- |
| `enqueue_audio(data, pts_us, sample_rate, channels, format)` | 音声データをキューに追加 |
| `play()` | 再生開始 |
| `pause()` | 一時停止 |
| `stop()` | 停止してキューをクリア |
| `stats()` | 統計情報を取得 |

| プロパティ | 説明 |
| --- | --- |
| `is_playing()` | 再生中か |
| `volume()` | 音量 (0.0 - 1.0) |
| `set_volume(volume)` | 音量を設定 |

### 低レベル API

ウィンドウ、レンダラー、テクスチャを直接操作する場合に使用。

```rust
use raw_player::{init, quit, poll_event, Event, Window, Renderer};

fn main() -> raw_player::Result<()> {
    init()?;

    let window = Window::new("Example", 640, 480)?;
    let renderer = Renderer::new(&window)?;

    loop {
        while let Some(event) = poll_event() {
            if matches!(event, Event::Quit) {
                // SDL リソースをすべて解放してから quit を呼ぶ
                drop(renderer);
                drop(window);
                // Safety: renderer, window は直前の drop で解放済み
                unsafe { quit() };
                return Ok(());
            }
        }
    }
}
```

## サンプル

### player

カメラ映像とマイク音声をキャプチャして AV 同期再生するサンプル。
`shiguredo_video_device` と `shiguredo_audio_device` を使用する。

```bash
# デバイス一覧を表示
cargo run --example player -- --list-devices

# デフォルト設定で再生 (720p, 30fps)
cargo run --example player

# 解像度とフレームレートを指定
cargo run --example player -- --resolution 1080p --fps 60

# 任意の解像度を指定
cargo run --example player -- --resolution 1920x1080

# デバイスを指定して再生
cargo run --example player -- --video-input-device <id> --audio-input-device <id>

# 再生時間を指定
cargo run --example player -- --duration 30
```

操作:

- ESC キーで終了
- S キーで統計オーバーレイ切替

## Simple DirectMedia Layer ライセンス

Zlib license

<https://github.com/libsdl-org/SDL/blob/main/LICENSE.txt>

```text
Copyright (C) 1997-2026 Sam Lantinga <slouken@libsdl.org>

This software is provided 'as-is', without any express or implied
warranty.  In no event will the authors be held liable for any damages
arising from the use of this software.

Permission is granted to anyone to use this software for any purpose,
including commercial applications, and to alter it and redistribute it
freely, subject to the following restrictions:

1. The origin of this software must not be misrepresented; you must not
   claim that you wrote the original software. If you use this software
   in a product, an acknowledgment in the product documentation would be
   appreciated but is not required.
2. Altered source versions must be plainly marked as such, and must not be
   misrepresented as being the original software.
3. This notice may not be removed or altered from any source distribution.
```

## raw-player-rs ライセンス

Apache License 2.0

```text
Copyright 2026-2026, Shiguredo Inc.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
