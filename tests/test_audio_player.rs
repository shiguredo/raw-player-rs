//! AudioPlayer の公開 API に対する単体テスト。
//!
//! play / pause / stop の成功経路が破綻しないことを確認する。
//! SDL 失敗時の原子性はモック禁止のためここでは証明しない。

use raw_player::{AudioFormat, AudioPlayer, init};

fn setup_sdl() {
    // Safety: テストプロセス内・SDL 初期化前のみ。
    unsafe {
        std::env::set_var("SDL_AUDIODRIVER", "dummy");
        std::env::set_var("SDL_VIDEODRIVER", "dummy");
    }
    init().expect("SDL init に失敗した");
}

fn silence_s16(sample_rate: i32, channels: i32, duration_ms: usize) -> Vec<u8> {
    let frames = sample_rate as usize * duration_ms / 1000;
    vec![0u8; frames * channels as usize * 2]
}

/// play → pause → stop → 再 enqueue → 再 play の成功経路を確認する。
#[test]
fn play_pause_stop_success_path_allows_replay() {
    setup_sdl();
    let player = AudioPlayer::new();
    let sample_rate = 48_000;
    let channels = 2;
    let data = silence_s16(sample_rate, channels, 100);

    player
        .enqueue_audio(&data, 0, sample_rate, channels, AudioFormat::S16)
        .expect("初回 enqueue に失敗した");
    player.play().expect("play に失敗した");
    assert!(player.is_playing(), "play 後は playing であるべき");

    player.pause().expect("pause に失敗した");
    assert!(!player.is_playing(), "pause 後は playing でないべき");

    player.stop().expect("stop に失敗した");
    assert!(!player.is_playing(), "stop 後は playing でないべき");

    // stop 後は再度 enqueue して play できる
    player
        .enqueue_audio(&data, 0, sample_rate, channels, AudioFormat::S16)
        .expect("stop 後の enqueue に失敗した");
    player.play().expect("再 play に失敗した");
    assert!(player.is_playing(), "再 play 後は playing であるべき");
}
