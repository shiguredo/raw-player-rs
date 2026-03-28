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

## 修正方針

SDL3 のドキュメントで `SDL_Renderer` / `SDL_Window` のスレッドセーフ性を確認する。

- スレッドセーフである場合: SAFETY コメントを正確な根拠に書き直す
- スレッドアフィニティが必要な場合: `Send` を外し、アーキテクチャを見直す
