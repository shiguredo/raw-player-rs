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

## SDL3 ソース確認結果

`SDL3/SDL_render.h` および `SDL3/SDL_video.h` の全関数に `\threadsafety This function should only be called on the main thread.` と明記されている。SDL3 でもスレッドアフィニティの要件は SDL2 と同一であり、メインスレッドからのみ操作すべき。

したがって、現状の `unsafe impl Send` は正当化できない。

## 修正方針

`Send` を外し、`VideoPlayer` のアーキテクチャを見直す必要がある。`Renderer` と `Window` から `Send` を外すと `VideoPlayerInner` が `Send` でなくなり、`Mutex<VideoPlayerInner>` が `Send + Sync` でなくなるため、`VideoPlayer` を別スレッドに移動できなくなる。設計変更の影響範囲が大きいため pending とする。

## pending の理由

修正にはアーキテクチャの見直しが必要で、単純なバグ修正では済まない。`VideoPlayer` の生成と操作をメインスレッドに限定する設計への変更が必要だが、利用側への影響も大きいため慎重に検討する。
