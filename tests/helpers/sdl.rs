//! SDL を使う統合テスト向けの共有ヘルパ。
//!
//! SDL はプロセス全体でスレッドセーフではないため、同一テストバイナリ内の
//! 並列実行を Mutex で直列化する。

use std::sync::{Mutex, MutexGuard};

use raw_player::init;

static SDL_TEST_LOCK: Mutex<()> = Mutex::new(());

/// SDL 利用区間を直列化するためのガード。
///
/// ウィンドウ / レンダラ / テクスチャ等の生存期間中は保持し続けること。
pub type SdlTestGuard = MutexGuard<'static, ()>;

/// ダミードライバを設定し SDL を初期化したうえで、プロセス内ロックを返す。
pub fn acquire_sdl() -> SdlTestGuard {
    let guard = SDL_TEST_LOCK
        .lock()
        .expect("SDL テストロックの取得に失敗した");
    // Safety: ロック保持下・他の SDL テストと同時に走らない。
    unsafe {
        std::env::set_var("SDL_VIDEODRIVER", "dummy");
        std::env::set_var("SDL_AUDIODRIVER", "dummy");
    }
    init().expect("SDL init に失敗した");
    guard
}
