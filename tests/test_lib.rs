//! `init` / `init_without_signal_handlers` の SDL シグナルハンドラ制御の退行テスト。

use std::ffi::CStr;

use raw_player::sys::{SDL_GetHint, SDL_HINT_NO_SIGNAL_HANDLERS};
use raw_player::{init, init_without_signal_handlers, quit};

/// POSIX の SIGTERM のシグナル番号。
#[cfg(unix)]
const SIGTERM: std::ffi::c_int = 15;

/// `signal(2)` が返す SIG_DFL (既定のハンドラ)。
#[cfg(unix)]
const SIG_DFL: usize = 0;

#[cfg(unix)]
unsafe extern "C" {
    /// シグナルハンドラを設定し、以前のハンドラを返す。
    ///
    /// SDL3 が SIGTERM のハンドラを登録したかどうかを判定するためだけに使う。
    fn signal(signum: std::ffi::c_int, handler: usize) -> usize;
}

/// 現在の SIGTERM ハンドラを取得する。
///
/// 現在のハンドラを問い合わせる API は `signal(2)` にないため、一時的にテスト用の
/// ハンドラを設定し、その戻り値 (以前のハンドラ) を取得してから元に戻す。
#[cfg(unix)]
fn sigterm_handler() -> usize {
    /// `signal(2)` はハンドラの型を検査しないため、テスト用の空ハンドラを置く。
    extern "C" fn marker(_signum: std::ffi::c_int) {}
    // `signal(2)` に渡すための、ハンドラのアドレス。
    let marker_address = marker as *const () as usize;

    // Safety: シグナルを送る処理はなく、ハンドラの設定と復元だけを同期的に行う。
    let previous = unsafe { signal(SIGTERM, marker_address) };
    assert_ne!(previous, usize::MAX, "SIGTERM ハンドラの取得に失敗した");
    // Safety: 取得した以前のハンドラに戻す。
    let restored = unsafe { signal(SIGTERM, previous) };
    assert_eq!(restored, marker_address, "SIGTERM ハンドラの復元に失敗した");
    previous
}

/// SDL のヒント `SDL_NO_SIGNAL_HANDLERS` の値を取得する。未設定なら `None`。
fn hint_value() -> Option<String> {
    // Safety: SDL_GetHint は SDL の初期化前でも呼べる。
    let value = unsafe { SDL_GetHint(SDL_HINT_NO_SIGNAL_HANDLERS.as_ptr().cast()) };
    if value.is_null() {
        return None;
    }
    // Safety: SDL_GetHint は NUL 終端された文字列を返す。
    Some(
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned(),
    )
}

/// `init()` は SDL のシグナルハンドラを登録し、`init_without_signal_handlers()` は登録しない。
#[test]
fn init_without_signal_handlers_disables_sdl_signal_handlers() {
    // SDL はプロセス全体の状態を持つため、このテストバイナリでは 1 つのテスト関数で
    // 初期化と終了を順番に呼び、並列実行による干渉を避ける。
    //
    // Safety: テストバイナリの起動直後で、このテスト以外に環境変数や SDL を使うスレッドはいない。
    unsafe {
        // 環境変数でヒントが上書きされると結果が変わってしまうため、明示的に消す。
        std::env::remove_var("SDL_NO_SIGNAL_HANDLERS");
        // CI などディスプレイのない環境でも初期化できるようダミードライバを使う。
        std::env::set_var("SDL_VIDEODRIVER", "dummy");
        std::env::set_var("SDL_AUDIODRIVER", "dummy");
    }

    // 既定の init() は SDL のシグナルハンドラを登録する。
    init().expect("既定の init() に失敗した");
    assert_eq!(hint_value(), None, "既定の init() はヒントを設定しない");
    #[cfg(unix)]
    assert_ne!(
        sigterm_handler(),
        SIG_DFL,
        "既定の init() は SDL の SIGTERM ハンドラを登録する"
    );

    // 初期化済みの状態で呼んでも何もしない。
    init_without_signal_handlers().expect("初期化済みで init_without_signal_handlers() が失敗した");
    assert_eq!(hint_value(), None, "初期化済みの場合はヒントを設定しない");
    #[cfg(unix)]
    assert_ne!(
        sigterm_handler(),
        SIG_DFL,
        "初期化済みの場合はシグナルハンドラを変更しない"
    );

    // Safety: SDL リソース (ウィンドウやレンダラー) は作成していない。
    unsafe { quit() };

    // init_without_signal_handlers() は SDL のシグナルハンドラを登録しない。
    init_without_signal_handlers().expect("init_without_signal_handlers() に失敗した");
    assert_eq!(
        hint_value().as_deref(),
        Some("1"),
        "init_without_signal_handlers() はヒントを 1 に設定する"
    );
    #[cfg(unix)]
    assert_eq!(
        sigterm_handler(),
        SIG_DFL,
        "init_without_signal_handlers() は SDL の SIGTERM ハンドラを登録しない"
    );

    // Safety: SDL リソース (ウィンドウやレンダラー) は作成していない。
    unsafe { quit() };
}
