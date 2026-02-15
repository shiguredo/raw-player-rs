use proptest::prelude::*;
use raw_player::{AudioFormat, AudioPlayer};

/// AudioFormat の Strategy
fn audio_format() -> impl Strategy<Value = AudioFormat> {
    prop_oneof![Just(AudioFormat::S16), Just(AudioFormat::F32),]
}

proptest! {
    #[test]
    fn rejects_empty_data(
        sample_rate in 1..=192000_i32,
        channels in 1..=8_i32,
        format in audio_format(),
    ) {
        let player = AudioPlayer::new();
        prop_assert!(player.enqueue_audio(&[], 0, sample_rate, channels, format).is_err());
    }

    #[test]
    fn rejects_non_positive_sample_rate(
        sample_rate in -100..=0_i32,
        channels in 1..=8_i32,
        format in audio_format(),
    ) {
        let player = AudioPlayer::new();
        let data = vec![0u8; 4];
        prop_assert!(player.enqueue_audio(&data, 0, sample_rate, channels, format).is_err());
    }

    #[test]
    fn rejects_non_positive_channels(
        sample_rate in 1..=192000_i32,
        channels in -100..=0_i32,
        format in audio_format(),
    ) {
        let player = AudioPlayer::new();
        let data = vec![0u8; 4];
        prop_assert!(player.enqueue_audio(&data, 0, sample_rate, channels, format).is_err());
    }

    #[test]
    fn rejects_unaligned_data(
        sample_rate in 1..=192000_i32,
        channels in 1..=8_i32,
        format in audio_format(),
    ) {
        let player = AudioPlayer::new();
        let frame_size = channels as usize * format.sample_size();
        // frame_size の倍数でないデータ (frame_size + 1 バイト)
        if frame_size > 0 {
            let data = vec![0u8; frame_size + 1];
            prop_assert!(player.enqueue_audio(&data, 0, sample_rate, channels, format).is_err());
        }
    }

    #[test]
    fn accepts_valid_data(
        sample_rate in 1..=192000_i32,
        channels in 1..=8_i32,
        format in audio_format(),
        num_frames in 1..=100_usize,
    ) {
        let player = AudioPlayer::new();
        let frame_size = channels as usize * format.sample_size();
        let data = vec![0u8; frame_size * num_frames];
        prop_assert!(player.enqueue_audio(&data, 0, sample_rate, channels, format).is_ok());
    }
}
