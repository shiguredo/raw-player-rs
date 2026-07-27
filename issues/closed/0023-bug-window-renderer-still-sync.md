# Window / Renderer が !Send だが Sync のまま残っている

- Priority: Medium
- Created: 2026-07-15
- Completed: 2026-07-17
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

コード変更は不要と判断し closed にした。

- 起票時の前提「`MutexGuard` は `Sync` なので `Window` / `Renderer` 全体も `Sync`」は、現行実装では成立しない
- `src/window.rs` / `src/renderer.rs` の `raw: NonNull<...>` により、既に `!Send` かつ `!Sync` である（std の `impl !Send for NonNull<T>` / `impl !Sync for NonNull<T>`）
- MSRV (`rust-version = "1.88"`) および 1.75 以降の rustc で同趣旨を実測した
- `assert_sync::<Window>()` / `assert_sync::<Renderer>()` / `assert_send::<&Window>()` 等はいずれも E0277 となり、完了条件の型契約は既に満たされている
- `PhantomData<MutexGuard<'static, ()>>` は 0003 由来の意図マーカーとして残るが、`!Sync` の実効因は `NonNull` である
