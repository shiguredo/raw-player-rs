# SDL の `init()` が並行初回呼び出しで完了前に成功を返し得る

Created: 2026-04-02
Model: Composer 2 Fast

## なぜ対応が必要か

`crate::init()` は「複数回呼び出しても安全」とドキュメントされているが、`SDL_INITIALIZED.swap(true)` の直後に別スレッドが `SDL_Init` の完了を待たず `Ok(())` を返し得る。未初期化のまま SDL API を呼ぶとクラッシュや未定義動作につながる。

## 根拠（コード）

`src/lib.rs`:

- `SDL_INITIALIZED.swap(true)` が `true` を返したスレッドは即 `return Ok(())`。
- 先に `swap` で `false` を立てたスレッドだけが `SDL_Init` を実行するため、2 スレッドが同時に初回呼び出しした場合、もう一方は初期化完了前に成功扱いになる。

## 期待する修正の方向性

- `std::sync::Once` / `OnceLock` 等で `SDL_Init` の成功まで他スレッドをブロックする、または
- 単一スレッドからのみ `init` / `VideoPlayer` を利用する旨を公開 API に明記し、並行初回をサポートしない。

## 関連

`issues/pending/0003-bug-unsafe-send-for-sdl-objects.md` の「メインスレッド限定」方針と整理すると重複が減る。
