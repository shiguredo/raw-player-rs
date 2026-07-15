# Window / Renderer が !Send だが Sync のまま残っている

- Priority: Medium
- Created: 2026-07-15
- Completed:
- Model: Grok 4.5
- Branch: feature/fix-window-renderer-still-sync
- Polished:

## 目的

`Window` / `Renderer` を `!Send + !Sync` にし、参照を他スレッドへ渡せないようにする。

## 優先度根拠

SDL のウィンドウ／レンダラはメインスレッド前提。`Send` は既に抑止されているが、`Sync` が残ると `&Window` / `&Renderer` を他スレッドへ渡せる。低レベル API として公開されているため型で防ぐ必要がある。

## 現状

`PhantomData<MutexGuard<'static, ()>>` により `!Send` になるが、`MutexGuard` は `Sync` のため型全体も `Sync` のまま。`VideoPlayer` は内部にこれらを持つため実質 `!Sync` だが、`Window` / `Renderer` を直接使う経路では防げない。

### 対象箇所

- `src/window.rs` の `_not_send`
- `src/renderer.rs` の `_not_send`

## 設計方針

`PhantomData<*const ()>` や `PhantomData<Rc<()>>` など、`!Send + !Sync` になるマーカーに置き換える。コメントでスレッドアフィニティの意図を日本語で残す。

## 完了条件

- `Window` / `Renderer` が `!Send` かつ `!Sync` である（コンパイル時に確認できる）
- 既存の単一スレッド利用は壊さない
- rustdoc またはコメントで契約が分かる

## 解決方法

1. PhantomData の型を `!Send + !Sync` に変更する
2. 必要なら静的アサーション（`static_assertions` 等を増やさず、doc テストやコメントでも可）で意図を固定する
3. 低レベル API のスレッド前提を rustdoc に追記する
