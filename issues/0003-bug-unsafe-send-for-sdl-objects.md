# Renderer と Window の unsafe impl Send の安全性主張が成立していない

Created: 2026-03-28
Model: Opus 4.6

## 概要

`Renderer` と `Window` に `unsafe impl Send` を付けているが、SAFETY コメントが「作成元スレッド以外からの操作を想定していない」と明記しており、安全性の根拠と主張が矛盾している。

## 根拠

`src/renderer.rs:195-197`:
```rust
// SAFETY: SDL_Renderer は作成元スレッド以外からの操作を想定していないが、
// Mutex<VideoPlayerInner> 内に保持し排他アクセスを保証しているため Send は安全。
unsafe impl Send for Renderer {}
```

`src/window.rs:63-65`:
```rust
// SAFETY: SDL_Window は Mutex<VideoPlayerInner> 内に保持し排他アクセスを保証しているため、
// 別スレッドへの移動は安全。
unsafe impl Send for Window {}
```

Mutex による排他制御は「同時アクセス」を防ぐだけで、「別スレッドから触ってよい」(スレッドアフィニティ) の証明にはならない。safe Rust の利用者は `Send` が付いている以上、別スレッドに移動しても安全だと信頼する。その信頼に対する根拠が存在しない。

## SDL3 のスレッド安全性（根拠の整理）

**誤り**: render / video 系の **すべて** の API がメインスレッド限定、という言い切りは不正確である。例として `SDL_GetRendererName` など、`\threadsafety It is safe to call this function from any thread.` とされる関数も存在する。

**本 issue で問題にする範囲**: 本クレートが `Window` / `Renderer` の生成・破棄に用いる API（`SDL_CreateWindow`、`SDL_DestroyWindow`、`SDL_CreateRenderer`、`SDL_DestroyRenderer` など）は、bindgen が参照する SDL3 ヘッダ上 **main thread のみ** と明記されている。ここに **`Mutex` だけではスレッドアフィニティの根拠にならない**。

したがって、`Window` / `Renderer` を **`Send` として別スレッドへ送れる**と主張する現状の `unsafe impl Send` は、少なくとも上記ライフサイクルに関する SDL の契約と整合しない。

## 修正方針

`Send` を外し、`VideoPlayer` のアーキテクチャを見直す。`Renderer` と `Window` から `Send` を外す（型レベルで `!Send` を表現する）と `VideoPlayerInner` が `Send` でなくなり、`Mutex<VideoPlayerInner>` も **`Send` を満たさなくなる**。

さらに **`Sync` も失う**（`T: Sync` が要るため）。その結果、

- **`VideoPlayer` 値を別スレッドへ `move` できなくなる**だけでなく、
- **別スレッドから `&VideoPlayer`（共有参照）越しにメソッドを呼ぶ**パターンも、型システム上で許されなくなる（`&VideoPlayer` が `Send` でないため、スレッド境界をまたぐ共有に使えない）。

「move できない」以上に、**クロススレッド共有そのもの**が止まる点が設計上の本質である。SDL の「メインスレッドでウィンドウ／レンダラを触る」要件と揃える。

## 破壊的変更について（方針決定）

後方互換を壊す変更（公開型の `Send` 喪失、API 契約の変更など）は **問題ない**。`CHANGES.md` では `[CHANGE]` として記載する。
