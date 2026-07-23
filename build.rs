use std::{
    path::{Path, PathBuf},
    process::Command,
};

// 依存ライブラリの名前
const LIB_NAME: &str = "sdl3";

fn main() {
    // Cargo.toml か build.rs が更新されたら、依存ライブラリを再ビルドする
    println!("cargo::rerun-if-changed=Cargo.toml");
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=CARGO_FEATURE_SOURCE_BUILD");
    println!("cargo::rerun-if-env-changed=DOCS_RS");

    // 各種変数やビルドディレクトリのセットアップ
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("infallible"));
    let output_metadata_path = out_dir.join("metadata.rs");
    let output_bindings_path = out_dir.join("bindings.rs");

    // 各種メタデータを書き込む
    let (url, tag) = get_url_and_tag();
    std::fs::write(
        output_metadata_path,
        format!(
            concat!(
                "pub const BUILD_METADATA_REPOSITORY: &str={:?};\n",
                "pub const BUILD_METADATA_VERSION: &str={:?};\n",
            ),
            url, tag
        ),
    )
    .expect("failed to write metadata file");

    if std::env::var("DOCS_RS").is_ok() {
        // Docs.rs 向け: curl / ネイティブビルド不可のため、src が参照する
        // bindings 記号・署名を揃えたダミーを出力し `cargo check` も通す。
        //
        // See also: https://docs.rs/about/builds
        std::fs::write(
            output_bindings_path,
            r#####"// Docs.rs 用ダミー bindings。src が参照する記号・署名を揃え cargo check を通す。
pub struct SDL_Window;
pub struct SDL_Renderer;
pub struct SDL_Texture;
pub struct SDL_AudioStream;
pub struct SDL_Rect;

pub type Uint8 = u8;
pub type Uint16 = u16;
pub type Uint32 = u32;
pub type Uint64 = u64;
pub type Sint32 = i32;
pub type SDL_InitFlags = u32;
pub type SDL_WindowFlags = u64;
pub type SDL_AudioDeviceID = u32;
pub type SDL_AudioFormat = ::std::os::raw::c_uint;
pub type SDL_PixelFormat = ::std::os::raw::c_uint;
pub type SDL_TextureAccess = ::std::os::raw::c_uint;
pub type SDL_RendererLogicalPresentation = ::std::os::raw::c_uint;
pub type SDL_BlendMode = u32;
pub type SDL_Keycode = u32;
pub type SDL_EventType = u32;
pub type SDL_AudioStreamCallback = Option<
    unsafe extern "C" fn(
        *mut ::std::os::raw::c_void,
        *mut SDL_AudioStream,
        ::std::os::raw::c_int,
        ::std::os::raw::c_int,
    ),
>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SDL_AudioSpec {
    pub format: SDL_AudioFormat,
    pub channels: ::std::os::raw::c_int,
    pub freq: ::std::os::raw::c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SDL_FRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SDL_KeyboardEvent {
    pub type_: u32,
    pub key: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SDL_WindowEvent {
    pub type_: u32,
    pub data1: i32,
    pub data2: i32,
}

#[repr(C)]
pub union SDL_Event {
    pub type_: u32,
    pub key: SDL_KeyboardEvent,
    pub window: SDL_WindowEvent,
}

// イベント型定数
pub const SDL_EventType_SDL_EVENT_QUIT: u32 = 0x100;
pub const SDL_EventType_SDL_EVENT_KEY_DOWN: u32 = 0x300;
pub const SDL_EventType_SDL_EVENT_KEY_UP: u32 = 0x301;
pub const SDL_EventType_SDL_EVENT_WINDOW_RESIZED: u32 = 0x206;
pub const SDL_EventType_SDL_EVENT_WINDOW_CLOSE_REQUESTED: u32 = 0x210;
// キーコード定数
pub const SDLK_ESCAPE: u32 = 27;
pub const SDLK_S: u32 = 115;
// ブレンドモード定数
pub const SDL_BLENDMODE_BLEND: u32 = 1;
// デバッグテキスト定数
pub const SDL_DEBUG_TEXT_FONT_CHARACTER_SIZE: i32 = 8;
// 初期化フラグ
pub const SDL_INIT_VIDEO: u32 = 32;
pub const SDL_INIT_AUDIO: u32 = 16;
// 音声フォーマット
pub const SDL_AudioFormat_SDL_AUDIO_S16: SDL_AudioFormat = 32784;
pub const SDL_AudioFormat_SDL_AUDIO_F32: SDL_AudioFormat = 33056;
// ピクセルフォーマット
pub const SDL_PixelFormat_SDL_PIXELFORMAT_IYUV: SDL_PixelFormat = 1448433993;
pub const SDL_PixelFormat_SDL_PIXELFORMAT_NV12: SDL_PixelFormat = 842094158;
pub const SDL_PixelFormat_SDL_PIXELFORMAT_YUY2: SDL_PixelFormat = 1499870533;
pub const SDL_PixelFormat_SDL_PIXELFORMAT_RGBA8888: SDL_PixelFormat = 373694468;
pub const SDL_PixelFormat_SDL_PIXELFORMAT_ARGB8888: SDL_PixelFormat = 372645892;
// テクスチャアクセス
pub const SDL_TextureAccess_SDL_TEXTUREACCESS_STREAMING: SDL_TextureAccess = 1;
// 論理プレゼンテーション
pub const SDL_RendererLogicalPresentation_SDL_LOGICAL_PRESENTATION_LETTERBOX: SDL_RendererLogicalPresentation = 2;

pub fn SDL_Init(_flags: SDL_InitFlags) -> bool { false }
pub fn SDL_Quit() {}
pub fn SDL_GetError() -> *const ::std::os::raw::c_char { ::std::ptr::null() }
pub fn SDL_CreateWindow(
    _title: *const ::std::os::raw::c_char,
    _w: ::std::os::raw::c_int,
    _h: ::std::os::raw::c_int,
    _flags: SDL_WindowFlags,
) -> *mut SDL_Window { ::std::ptr::null_mut() }
pub fn SDL_DestroyWindow(_window: *mut SDL_Window) {}
pub fn SDL_GetWindowSize(
    _window: *mut SDL_Window,
    _w: *mut ::std::os::raw::c_int,
    _h: *mut ::std::os::raw::c_int,
) -> bool { false }
pub fn SDL_SetWindowSize(
    _window: *mut SDL_Window,
    _w: ::std::os::raw::c_int,
    _h: ::std::os::raw::c_int,
) -> bool { false }
pub fn SDL_SetWindowTitle(
    _window: *mut SDL_Window,
    _title: *const ::std::os::raw::c_char,
) -> bool { false }
pub fn SDL_CreateRenderer(
    _window: *mut SDL_Window,
    _name: *const ::std::os::raw::c_char,
) -> *mut SDL_Renderer { ::std::ptr::null_mut() }
pub fn SDL_DestroyRenderer(_renderer: *mut SDL_Renderer) {}
pub fn SDL_RenderClear(_renderer: *mut SDL_Renderer) -> bool { false }
pub fn SDL_RenderPresent(_renderer: *mut SDL_Renderer) -> bool { false }
pub fn SDL_RenderTexture(
    _renderer: *mut SDL_Renderer,
    _texture: *mut SDL_Texture,
    _srcrect: *const SDL_FRect,
    _dstrect: *const SDL_FRect,
) -> bool { false }
pub fn SDL_SetRenderDrawColor(
    _renderer: *mut SDL_Renderer,
    _r: Uint8,
    _g: Uint8,
    _b: Uint8,
    _a: Uint8,
) -> bool { false }
pub fn SDL_SetRenderDrawBlendMode(
    _renderer: *mut SDL_Renderer,
    _blend_mode: SDL_BlendMode,
) -> bool { false }
pub fn SDL_RenderFillRect(
    _renderer: *mut SDL_Renderer,
    _rect: *const SDL_FRect,
) -> bool { false }
pub fn SDL_RenderDebugText(
    _renderer: *mut SDL_Renderer,
    _x: f32,
    _y: f32,
    _str: *const ::std::os::raw::c_char,
) -> bool { false }
pub fn SDL_GetRenderOutputSize(
    _renderer: *mut SDL_Renderer,
    _w: *mut ::std::os::raw::c_int,
    _h: *mut ::std::os::raw::c_int,
) -> bool { false }
pub fn SDL_GetRenderScale(
    _renderer: *mut SDL_Renderer,
    _scale_x: *mut f32,
    _scale_y: *mut f32,
) -> bool { false }
pub fn SDL_SetRenderScale(
    _renderer: *mut SDL_Renderer,
    _scale_x: f32,
    _scale_y: f32,
) -> bool { false }
pub fn SDL_GetRendererName(_renderer: *mut SDL_Renderer) -> *const ::std::os::raw::c_char {
    ::std::ptr::null()
}
pub fn SDL_SetRenderLogicalPresentation(
    _renderer: *mut SDL_Renderer,
    _w: ::std::os::raw::c_int,
    _h: ::std::os::raw::c_int,
    _mode: SDL_RendererLogicalPresentation,
) -> bool { false }
pub fn SDL_SetRenderVSync(
    _renderer: *mut SDL_Renderer,
    _vsync: ::std::os::raw::c_int,
) -> bool { false }
pub fn SDL_CreateTexture(
    _renderer: *mut SDL_Renderer,
    _format: SDL_PixelFormat,
    _access: SDL_TextureAccess,
    _w: ::std::os::raw::c_int,
    _h: ::std::os::raw::c_int,
) -> *mut SDL_Texture { ::std::ptr::null_mut() }
pub fn SDL_DestroyTexture(_texture: *mut SDL_Texture) {}
pub fn SDL_UpdateYUVTexture(
    _texture: *mut SDL_Texture,
    _rect: *const SDL_Rect,
    _yplane: *const Uint8,
    _ypitch: ::std::os::raw::c_int,
    _uplane: *const Uint8,
    _upitch: ::std::os::raw::c_int,
    _vplane: *const Uint8,
    _vpitch: ::std::os::raw::c_int,
) -> bool { false }
pub fn SDL_UpdateNVTexture(
    _texture: *mut SDL_Texture,
    _rect: *const SDL_Rect,
    _yplane: *const Uint8,
    _ypitch: ::std::os::raw::c_int,
    _uvplane: *const Uint8,
    _uvpitch: ::std::os::raw::c_int,
) -> bool { false }
pub fn SDL_UpdateTexture(
    _texture: *mut SDL_Texture,
    _rect: *const SDL_Rect,
    _pixels: *const ::std::os::raw::c_void,
    _pitch: ::std::os::raw::c_int,
) -> bool { false }
pub fn SDL_OpenAudioDeviceStream(
    _devid: SDL_AudioDeviceID,
    _spec: *const SDL_AudioSpec,
    _callback: SDL_AudioStreamCallback,
    _userdata: *mut ::std::os::raw::c_void,
) -> *mut SDL_AudioStream { ::std::ptr::null_mut() }
pub fn SDL_DestroyAudioStream(_stream: *mut SDL_AudioStream) {}
pub fn SDL_PutAudioStreamData(
    _stream: *mut SDL_AudioStream,
    _buf: *const ::std::os::raw::c_void,
    _len: ::std::os::raw::c_int,
) -> bool { false }
pub fn SDL_GetAudioStreamQueued(_stream: *mut SDL_AudioStream) -> ::std::os::raw::c_int { 0 }
pub fn SDL_PauseAudioStreamDevice(_stream: *mut SDL_AudioStream) -> bool { false }
pub fn SDL_ResumeAudioStreamDevice(_stream: *mut SDL_AudioStream) -> bool { false }
pub fn SDL_ClearAudioStream(_stream: *mut SDL_AudioStream) -> bool { false }
pub fn SDL_SetAudioStreamGain(_stream: *mut SDL_AudioStream, _gain: f32) -> bool { false }
pub fn SDL_GetTicksNS() -> Uint64 { 0 }
pub fn SDL_PollEvent(_event: *mut SDL_Event) -> bool { false }
"#####,
        )
        .expect("write file error");
        return;
    }

    let output_lib_dir = if should_use_prebuilt() {
        download_prebuilt(&out_dir, &output_bindings_path)
    } else {
        build_from_source(&out_dir, &output_bindings_path, &url, &tag)
    };

    // リンク設定
    println!(
        "cargo::rustc-link-search=native={}",
        output_lib_dir.display()
    );

    // Windows では SDL3-static.lib、Linux/macOS では libSDL3.a としてインストールされる
    if output_lib_dir.join("SDL3-static.lib").exists() {
        println!("cargo::rustc-link-lib=static=SDL3-static");
    } else {
        println!("cargo::rustc-link-lib=static=SDL3");
    }

    // macOS frameworks
    #[cfg(target_os = "macos")]
    {
        println!("cargo::rustc-link-lib=framework=Cocoa");
        println!("cargo::rustc-link-lib=framework=IOKit");
        println!("cargo::rustc-link-lib=framework=CoreFoundation");
        println!("cargo::rustc-link-lib=framework=CoreVideo");
        println!("cargo::rustc-link-lib=framework=Metal");
        println!("cargo::rustc-link-lib=framework=QuartzCore");
        println!("cargo::rustc-link-lib=framework=GameController");
        println!("cargo::rustc-link-lib=framework=CoreHaptics");
        println!("cargo::rustc-link-lib=framework=ForceFeedback");
        println!("cargo::rustc-link-lib=framework=Carbon");
        println!("cargo::rustc-link-lib=framework=CoreAudio");
        println!("cargo::rustc-link-lib=framework=AudioToolbox");
        println!("cargo::rustc-link-lib=framework=AVFoundation");
        println!("cargo::rustc-link-lib=framework=CoreMedia");
        println!("cargo::rustc-link-lib=framework=UniformTypeIdentifiers");
        println!("cargo::rustc-link-lib=iconv");
    }

    // Windows libraries
    #[cfg(target_os = "windows")]
    {
        for lib in [
            "user32", "gdi32", "winmm", "imm32", "ole32", "oleaut32", "version", "uuid",
            "setupapi", "shell32", "advapi32", "cfgmgr32",
        ] {
            println!("cargo::rustc-link-lib={lib}");
        }
    }

    // Linux libraries
    #[cfg(target_os = "linux")]
    {
        println!("cargo::rustc-link-lib=pthread");
        println!("cargo::rustc-link-lib=dl");
        println!("cargo::rustc-link-lib=m");
    }
}

// source-build feature が有効でなければ prebuilt を使用する
fn should_use_prebuilt() -> bool {
    std::env::var("CARGO_FEATURE_SOURCE_BUILD").is_err()
}

// prebuilt バイナリをダウンロードして配置する
fn download_prebuilt(out_dir: &Path, output_bindings_path: &Path) -> PathBuf {
    let platform = get_target_platform();
    let version = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION is not set");
    let base_url =
        format!("https://github.com/shiguredo/raw-player-rs/releases/download/{version}");
    let archive_name = format!("libSDL3-{platform}.tar.gz");
    let checksum_name = format!("{archive_name}.sha256");

    let archive_url = format!("{base_url}/{archive_name}");
    let checksum_url = format!("{base_url}/{checksum_name}");

    let prebuilt_dir = out_dir.join("prebuilt");
    let _ = std::fs::remove_dir_all(&prebuilt_dir);
    std::fs::create_dir_all(&prebuilt_dir).expect("failed to create prebuilt directory");

    let archive_path = prebuilt_dir.join(&archive_name);
    let checksum_path = prebuilt_dir.join(&checksum_name);

    // アーカイブをダウンロード
    println!("Downloading prebuilt SDL3 from {archive_url}");
    curl_download(&archive_url, &archive_path);

    // チェックサムをダウンロード
    curl_download(&checksum_url, &checksum_path);

    // SHA256 を検証
    let expected_hash = std::fs::read_to_string(&checksum_path)
        .expect("failed to read checksum file")
        .split_whitespace()
        .next()
        .expect("empty checksum file")
        .to_string();
    verify_sha256(&archive_path, &expected_hash);

    // 展開
    extract_tar_gz(&archive_path, &prebuilt_dir);

    // lib ディレクトリにコピー
    let lib_dir = out_dir.join("lib");
    let _ = std::fs::remove_dir_all(&lib_dir);
    std::fs::create_dir_all(&lib_dir).expect("failed to create lib directory");

    let lib_filename = get_lib_filename();
    std::fs::copy(
        prebuilt_dir.join(&lib_filename),
        lib_dir.join(&lib_filename),
    )
    .unwrap_or_else(|e| panic!("failed to copy {lib_filename}: {e}"));

    // bindings.rs をコピー
    std::fs::copy(prebuilt_dir.join("bindings.rs"), output_bindings_path)
        .expect("failed to copy bindings.rs");

    // ダウンロードしたファイルを削除
    let _ = std::fs::remove_dir_all(&prebuilt_dir);

    lib_dir
}

// ソースからビルドする
fn build_from_source(out_dir: &Path, output_bindings_path: &Path, url: &str, tag: &str) -> PathBuf {
    let src_dir = out_dir.join("SDL");

    // git clone でソースを取得する（キャッシュ機構: CMakeLists.txt が存在しない場合のみ）
    if !src_dir.join("CMakeLists.txt").exists() {
        git_clone(url, tag, &src_dir);
    }

    // shiguredo_cmake が管理する CMake バイナリを使用する
    shiguredo_cmake::set_cmake_env();

    // profile("Release") を使用: Windows の Visual Studio ジェネレーター (マルチ構成) では
    // CMAKE_BUILD_TYPE が無視されるため、cmake crate の profile() で統一的に指定する
    let dst = shiguredo_cmake::Config::new(&src_dir)
        .define("SDL_STATIC", "ON")
        .define("SDL_SHARED", "OFF")
        .define("SDL_TEST_LIBRARY", "OFF")
        .profile("Release")
        // 不要な機能を無効化
        .define("SDL_HAPTIC", "OFF")
        .define("SDL_HIDAPI", "OFF")
        .define("SDL_POWER", "OFF")
        .define("SDL_SENSOR", "OFF")
        .define("SDL_DIALOG", "OFF")
        .define("SDL_CAMERA", "OFF")
        .define("SDL_X11_XCURSOR", "OFF")
        .define("SDL_X11_XDBE", "OFF")
        .define("SDL_X11_XSCRNSAVER", "OFF")
        .define("SDL_X11_XSHAPE", "OFF")
        .define("SDL_X11_XSYNC", "OFF")
        .define("SDL_X11_XTEST", "OFF")
        .build();

    let include_dir = dst.join("include");

    // バインディングを生成する
    bindgen::Builder::default()
        .header(include_dir.join("SDL3/SDL.h").to_str().unwrap())
        .clang_arg(format!("-I{}", include_dir.display()))
        // 初期化と終了
        .allowlist_function("SDL_Init")
        .allowlist_function("SDL_Quit")
        .allowlist_function("SDL_GetError")
        // ウィンドウ
        .allowlist_function("SDL_CreateWindow")
        .allowlist_function("SDL_DestroyWindow")
        .allowlist_function("SDL_GetWindowSize")
        .allowlist_function("SDL_SetWindowSize")
        .allowlist_function("SDL_SetWindowTitle")
        // レンダラー
        .allowlist_function("SDL_CreateRenderer")
        .allowlist_function("SDL_DestroyRenderer")
        .allowlist_function("SDL_RenderClear")
        .allowlist_function("SDL_RenderPresent")
        .allowlist_function("SDL_RenderTexture")
        .allowlist_function("SDL_SetRenderDrawColor")
        .allowlist_function("SDL_SetRenderDrawBlendMode")
        .allowlist_function("SDL_RenderFillRect")
        .allowlist_function("SDL_RenderDebugText")
        .allowlist_function("SDL_GetRenderOutputSize")
        .allowlist_function("SDL_GetRenderScale")
        .allowlist_function("SDL_SetRenderScale")
        .allowlist_function("SDL_GetRendererName")
        // テクスチャ
        .allowlist_function("SDL_CreateTexture")
        .allowlist_function("SDL_DestroyTexture")
        .allowlist_function("SDL_UpdateYUVTexture")
        .allowlist_function("SDL_UpdateNVTexture")
        .allowlist_function("SDL_UpdateTexture")
        .allowlist_function("SDL_LockTexture")
        .allowlist_function("SDL_UnlockTexture")
        // レンダラー (追加)
        .allowlist_function("SDL_SetRenderLogicalPresentation")
        .allowlist_function("SDL_SetRenderVSync")
        // 音声 API
        .allowlist_function("SDL_OpenAudioDeviceStream")
        .allowlist_function("SDL_DestroyAudioStream")
        .allowlist_function("SDL_PutAudioStreamData")
        .allowlist_function("SDL_GetAudioStreamQueued")
        .allowlist_function("SDL_PauseAudioStreamDevice")
        .allowlist_function("SDL_ResumeAudioStreamDevice")
        .allowlist_function("SDL_ClearAudioStream")
        .allowlist_function("SDL_SetAudioStreamGain")
        // タイマー
        .allowlist_function("SDL_GetTicksNS")
        // イベント
        .allowlist_function("SDL_PollEvent")
        // 型
        .allowlist_type("SDL_Window")
        .allowlist_type("SDL_Renderer")
        .allowlist_type("SDL_Texture")
        .allowlist_type("SDL_Event")
        .allowlist_type("SDL_KeyboardEvent")
        .allowlist_type("SDL_WindowEvent")
        .allowlist_type("SDL_QuitEvent")
        .allowlist_type("SDL_FRect")
        .allowlist_type("SDL_AudioStream")
        .allowlist_type("SDL_AudioSpec")
        .allowlist_type("SDL_AudioDeviceID")
        // 定数
        .allowlist_var("SDL_INIT_VIDEO")
        .allowlist_var("SDL_INIT_AUDIO")
        .allowlist_var("SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK")
        .allowlist_var("SDL_AUDIO_F32")
        .allowlist_var("SDL_AUDIO_S16")
        .allowlist_var("SDL_PIXELFORMAT_NV12")
        .allowlist_var("SDL_PIXELFORMAT_YUY2")
        .allowlist_var("SDL_PIXELFORMAT_RGBA8888")
        .allowlist_var("SDL_PIXELFORMAT_ARGB8888")
        .allowlist_var("SDL_LOGICAL_PRESENTATION_LETTERBOX")
        .allowlist_var("SDL_EVENT_QUIT")
        .allowlist_var("SDL_EVENT_KEY_DOWN")
        .allowlist_var("SDL_EVENT_KEY_UP")
        .allowlist_var("SDL_EVENT_WINDOW_.*")
        .allowlist_var("SDL_PIXELFORMAT_IYUV")
        .allowlist_var("SDL_TEXTUREACCESS_.*")
        .allowlist_var("SDLK_.*")
        .allowlist_var("SDL_WINDOW_.*")
        .allowlist_var("SDL_BLENDMODE_.*")
        .allowlist_var("SDL_DEBUG_TEXT_FONT_CHARACTER_SIZE")
        .derive_default(true)
        .derive_debug(true)
        .impl_debug(false)
        .generate_comments(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(output_bindings_path)
        .expect("Couldn't write bindings!");

    dst.join("lib")
}

// git clone でソースを取得する
fn git_clone(url: &str, tag: &str, dest: &Path) {
    println!("cargo::warning=Cloning {url}");

    let success = Command::new("git")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg("--branch")
        .arg(tag)
        .arg(url)
        .arg(dest)
        .status()
        .is_ok_and(|status| status.success());

    if !success {
        panic!("failed to clone SDL3 from {url}");
    }
}

// curl でファイルをダウンロードする
fn curl_download(url: &str, dest: &Path) {
    let success = Command::new("curl")
        .arg("-fsSL")
        .arg("--retry")
        .arg("3")
        .arg("-o")
        .arg(dest)
        .arg(url)
        .status()
        .is_ok_and(|status| status.success());

    if !success {
        panic!("failed to download from {url}");
    }
}

// tar.gz を展開する
fn extract_tar_gz(archive: &Path, dest: &Path) {
    let success = Command::new("tar")
        .arg("-xzf")
        .arg(archive)
        .arg("-C")
        .arg(dest)
        .status()
        .is_ok_and(|status| status.success());

    if !success {
        panic!("failed to extract {}", archive.display());
    }
}

// OS コマンドを使って SHA256 ハッシュを計算する
fn compute_sha256(file_path: &Path) -> String {
    let output = if cfg!(target_os = "macos") {
        Command::new("shasum")
            .arg("-a")
            .arg("256")
            .arg(file_path)
            .output()
            .expect("failed to execute shasum")
    } else if cfg!(target_os = "windows") {
        Command::new("certutil")
            .arg("-hashfile")
            .arg(file_path)
            .arg("SHA256")
            .output()
            .expect("failed to execute certutil")
    } else {
        Command::new("sha256sum")
            .arg(file_path)
            .output()
            .expect("failed to execute sha256sum")
    };

    if !output.status.success() {
        panic!("SHA256 command failed for {}", file_path.display());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    if cfg!(target_os = "windows") {
        // certutil の出力形式: "SHA256 hash of <path>:\n<hash>\nCertUtil: ..."
        stdout
            .lines()
            .nth(1)
            .expect("unexpected certutil output")
            .replace(' ', "")
            .to_lowercase()
    } else {
        // shasum/sha256sum の出力形式: "<hash>  <filename>"
        stdout
            .split_whitespace()
            .next()
            .expect("unexpected sha256 output")
            .to_lowercase()
    }
}

// ファイルの SHA256 ハッシュを検証する
fn verify_sha256(file_path: &Path, expected_hash: &str) {
    println!("Verifying SHA256 hash for {}", file_path.display());

    let calculated_hash = compute_sha256(file_path);

    if calculated_hash.eq_ignore_ascii_case(expected_hash) {
        println!("=> SHA256 hash verified: {calculated_hash}");
    } else {
        panic!("SHA256 hash mismatch!\nExpected: {expected_hash}\nCalculated: {calculated_hash}");
    }
}

// ターゲット OS に応じたライブラリファイル名を返す
fn get_lib_filename() -> String {
    let os = std::env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS not set");
    if os == "windows" {
        "SDL3-static.lib".to_string()
    } else {
        "libSDL3.a".to_string()
    }
}

// ターゲットプラットフォームを判定する
fn get_target_platform() -> String {
    let os = std::env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS not set");
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").expect("CARGO_CFG_TARGET_ARCH not set");

    match (os.as_str(), arch.as_str()) {
        ("linux", "x86_64") => {
            let version_id = get_ubuntu_version_id();
            format!("ubuntu-{version_id}_x86_64")
        }
        ("linux", "aarch64") => {
            let version_id = get_ubuntu_version_id();
            format!("ubuntu-{version_id}_arm64")
        }
        ("macos", "aarch64") => "macos_arm64".to_string(),
        ("windows", "x86_64") => "windows_x86_64".to_string(),
        _ => {
            panic!("unsupported platform: {os}-{arch}");
        }
    }
}

// Ubuntu のバージョン ID を取得する
fn get_ubuntu_version_id() -> String {
    let content =
        std::fs::read_to_string("/etc/os-release").expect("failed to read /etc/os-release");
    for line in content.lines() {
        if let Some(version) = line.strip_prefix("VERSION_ID=") {
            return version.trim_matches('"').to_string();
        }
    }
    panic!("VERSION_ID not found in /etc/os-release");
}

// Cargo.toml をパースしてメタデータテーブルを取得する
fn get_metadata() -> shiguredo_toml::Value {
    shiguredo_toml::Value::Table(
        shiguredo_toml::from_str(include_str!("Cargo.toml")).expect("failed to parse Cargo.toml"),
    )
}

// Cargo.toml から依存ライブラリの URL とタグを取得する
fn get_url_and_tag() -> (String, String) {
    let cargo_toml = get_metadata();
    if let Some((Some(url), Some(tag))) = cargo_toml
        .get("package")
        .and_then(|v| v.get("metadata"))
        .and_then(|v| v.get("external-dependencies"))
        .and_then(|v| v.get(LIB_NAME))
        .map(|v| {
            (
                v.get("url").and_then(|s| s.as_str()),
                v.get("tag").and_then(|s| s.as_str()),
            )
        })
    {
        (url.to_string(), tag.to_string())
    } else {
        panic!(
            "Cargo.toml does not contain a valid [package.metadata.external-dependencies.{LIB_NAME}] table"
        );
    }
}
