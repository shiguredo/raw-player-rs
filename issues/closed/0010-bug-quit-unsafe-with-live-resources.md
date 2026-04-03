# quit() が live な SDL リソースの存在を確認せず SDL_Quit を呼べる

Created: 2026-04-03
Completed: 2026-04-03
Model: Opus 4.6

## 概要

`pub fn quit()` は safe API だが、`VideoPlayer`, `Window`, `Renderer` 等の SDL リソースが live な状態でも呼び出せてしまう。`SDL_Quit` 後にこれらのリソースが drop されると、解放済みリソースへのアクセスでクラッシュする。

## 根拠

- `quit()` は `pub fn` であり、呼び出し側に事前条件の遵守を要求している
- Rust の safe API は未定義動作を起こさないことが期待される
- ドキュメントコメントで「先に drop せよ」と書いてあるだけでコンパイラレベルの保護がない

## 対応方針

`quit()` を非公開化し、代わりに `init()` が返す RAII ガードの `Drop` で自動的に `SDL_Quit` を呼ぶ設計に変更する。ガードが live な間は `SDL_Quit` が呼ばれないことを型レベルで保証する。

ただし、この変更は API の破壊的変更を伴うため、段階的に対応する。まず `quit()` を deprecated にして unsafe であることを明示し、将来的に RAII ガード方式へ移行する。

## 解決方法

`quit()` を `pub unsafe fn` に変更し、呼び出し側に Safety 契約の遵守を明示的に要求するようにした。ドキュメントに `# Safety` セクションを追加した。既存の呼び出し元 (examples/player.rs, テストコード) を `unsafe` ブロックで囲み、Safety コメントを付与した。
