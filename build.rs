use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

// 依存ライブラリの名前
const LIB_NAME: &str = "sdl3";

fn git_clone(url: &str, dest: &Path, tag: Option<&str>) {
    println!("cargo::warning=Cloning {url}");
    let status = Command::new("git")
        .args(["clone", url])
        .arg(dest)
        .status()
        .unwrap_or_else(|e| panic!("Failed to execute git clone: {e}"));
    assert!(status.success(), "git clone failed for {url}");

    if let Some(tag_name) = tag {
        let status = Command::new("git")
            .args(["checkout", &format!("refs/tags/{}", tag_name)])
            .current_dir(dest)
            .status()
            .unwrap_or_else(|e| panic!("Failed to execute git checkout: {e}"));
        assert!(status.success(), "git checkout failed for {tag_name}");
    }
}

// Cargo.toml から依存ライブラリの Git URL とタグを取得する
fn get_git_url_and_tag() -> (String, String) {
    use shiguredo_toml::PathSegment::Key;

    let doc = shiguredo_toml::Document::parse(include_str!("Cargo.toml"))
        .expect("failed to parse Cargo.toml");

    let git_url = doc
        .get(&[
            Key("package".into()),
            Key("metadata".into()),
            Key("external-dependencies".into()),
            Key(LIB_NAME.into()),
            Key("git".into()),
        ])
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            panic!(
                "Cargo.toml does not contain a valid [package.metadata.external-dependencies.{LIB_NAME}] table"
            )
        });

    let tag = doc
        .get(&[
            Key("package".into()),
            Key("metadata".into()),
            Key("external-dependencies".into()),
            Key(LIB_NAME.into()),
            Key("tag".into()),
        ])
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            panic!(
                "Cargo.toml does not contain a valid [package.metadata.external-dependencies.{LIB_NAME}] table"
            )
        });

    (git_url.to_string(), tag.to_string())
}

fn main() {
    println!("cargo::rerun-if-changed=Cargo.toml");
    println!("cargo::rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let sdl3_dir = out_dir.join("SDL");

    // Clone SDL3 if not present
    let (git_url, tag) = get_git_url_and_tag();
    if !sdl3_dir.join("CMakeLists.txt").exists() {
        git_clone(&git_url, &sdl3_dir, Some(&tag));
    }

    // Build SDL3 with cmake
    // shiguredo_cmake がダウンロード済みバイナリを CMAKE 環境変数に設定する
    shiguredo_cmake::set_cmake_env();
    // profile("Release") を使用: Windows の Visual Studio ジェネレーター (マルチ構成) では
    // CMAKE_BUILD_TYPE が無視されるため、cmake crate の profile() で統一的に指定する
    let dst = shiguredo_cmake::Config::new(&sdl3_dir)
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
        .define("SDL_X11_XINPUT", "OFF")
        .define("SDL_X11_XSCRNSAVER", "OFF")
        .define("SDL_X11_XSHAPE", "OFF")
        .define("SDL_X11_XSYNC", "OFF")
        .define("SDL_X11_XTEST", "OFF")
        .build();

    // Link the library
    let lib_dir = dst.join("lib");
    println!("cargo::rustc-link-search=native={}", lib_dir.display());

    // Windows では SDL3-static.lib、Linux/macOS では libSDL3.a としてインストールされる
    if lib_dir.join("SDL3-static.lib").exists() {
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

    let include_dir = dst.join("include");

    // Generate bindings
    let bindings = bindgen::Builder::default()
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
        .expect("Unable to generate bindings");

    let bindings_path = out_dir.join("bindings.rs");
    bindings
        .write_to_file(&bindings_path)
        .expect("Couldn't write bindings!");
}
