use proptest::prelude::*;
use raw_player::{validate_bgra, validate_i420, validate_nv12, validate_rgba, validate_yuy2};

/// 偶数の幅と高さを生成する Strategy
fn even_dim() -> impl Strategy<Value = i32> {
    (1..=512_i32).prop_map(|v| v * 2)
}

/// 正の幅と高さを生成する Strategy
fn positive_dim() -> impl Strategy<Value = i32> {
    1..=1024_i32
}

proptest! {
    #[test]
    fn valid_i420_accepts_correct_size(w in even_dim(), h in even_dim()) {
        let y = vec![0u8; (w * h) as usize];
        let uv_size = ((w / 2) * (h / 2)) as usize;
        let u = vec![0u8; uv_size];
        let v = vec![0u8; uv_size];
        prop_assert!(validate_i420(&y, &u, &v, w, h).is_ok());
    }

    #[test]
    fn i420_rejects_odd_width(h in even_dim()) {
        let w = 3; // 奇数
        let y = vec![0u8; (w * h) as usize];
        let u = vec![0u8; 1];
        let v = vec![0u8; 1];
        prop_assert!(validate_i420(&y, &u, &v, w, h).is_err());
    }

    #[test]
    fn i420_rejects_odd_height(w in even_dim()) {
        let h = 3; // 奇数
        let y = vec![0u8; (w * h) as usize];
        let u = vec![0u8; 1];
        let v = vec![0u8; 1];
        prop_assert!(validate_i420(&y, &u, &v, w, h).is_err());
    }

    #[test]
    fn i420_rejects_wrong_y_size(w in even_dim(), h in even_dim()) {
        let y = vec![0u8; (w * h) as usize + 1]; // 1 バイト多い
        let uv_size = ((w / 2) * (h / 2)) as usize;
        let u = vec![0u8; uv_size];
        let v = vec![0u8; uv_size];
        prop_assert!(validate_i420(&y, &u, &v, w, h).is_err());
    }

    #[test]
    fn valid_nv12_accepts_correct_size(w in even_dim(), h in even_dim()) {
        let y = vec![0u8; (w * h) as usize];
        let uv = vec![0u8; (w * (h / 2)) as usize];
        prop_assert!(validate_nv12(&y, &uv, w, h).is_ok());
    }

    #[test]
    fn nv12_rejects_odd_dimensions(w in even_dim()) {
        let h = 5; // 奇数
        let y = vec![0u8; (w * h) as usize];
        let uv = vec![0u8; 1];
        prop_assert!(validate_nv12(&y, &uv, w, h).is_err());
    }

    #[test]
    fn nv12_rejects_wrong_uv_size(w in even_dim(), h in even_dim()) {
        let y = vec![0u8; (w * h) as usize];
        let uv = vec![0u8; (w * (h / 2)) as usize + 1]; // 1 バイト多い
        prop_assert!(validate_nv12(&y, &uv, w, h).is_err());
    }

    #[test]
    fn valid_yuy2_accepts_correct_size(w in even_dim(), h in positive_dim()) {
        let data = vec![0u8; (w * h * 2) as usize];
        prop_assert!(validate_yuy2(&data, w, h).is_ok());
    }

    #[test]
    fn yuy2_rejects_odd_width(h in positive_dim()) {
        let w = 3; // 奇数
        let data = vec![0u8; (w * h * 2) as usize];
        prop_assert!(validate_yuy2(&data, w, h).is_err());
    }

    #[test]
    fn yuy2_rejects_wrong_size(w in even_dim(), h in positive_dim()) {
        let data = vec![0u8; (w * h * 2) as usize + 1]; // 1 バイト多い
        prop_assert!(validate_yuy2(&data, w, h).is_err());
    }

    #[test]
    fn valid_rgba_accepts_correct_size(w in positive_dim(), h in positive_dim()) {
        let data = vec![0u8; (w * h * 4) as usize];
        prop_assert!(validate_rgba(&data, w, h).is_ok());
    }

    #[test]
    fn rgba_rejects_wrong_size(w in positive_dim(), h in positive_dim()) {
        let data = vec![0u8; (w * h * 4) as usize + 1]; // 1 バイト多い
        prop_assert!(validate_rgba(&data, w, h).is_err());
    }

    #[test]
    fn all_validators_reject_zero_dimensions(data in proptest::collection::vec(any::<u8>(), 0..256)) {
        prop_assert!(validate_i420(&data, &data, &data, 0, 4).is_err());
        prop_assert!(validate_i420(&data, &data, &data, 4, 0).is_err());
        prop_assert!(validate_nv12(&data, &data, 0, 4).is_err());
        prop_assert!(validate_nv12(&data, &data, 4, 0).is_err());
        prop_assert!(validate_yuy2(&data, 0, 4).is_err());
        prop_assert!(validate_yuy2(&data, 4, 0).is_err());
        prop_assert!(validate_rgba(&data, 0, 4).is_err());
        prop_assert!(validate_rgba(&data, 4, 0).is_err());
    }

    #[test]
    fn all_validators_reject_negative_dimensions(data in proptest::collection::vec(any::<u8>(), 0..256)) {
        prop_assert!(validate_i420(&data, &data, &data, -2, 4).is_err());
        prop_assert!(validate_nv12(&data, &data, -2, 4).is_err());
        prop_assert!(validate_yuy2(&data, -2, 4).is_err());
        prop_assert!(validate_rgba(&data, -2, 4).is_err());
        prop_assert!(validate_bgra(&data, -2, 4).is_err());
    }

    #[test]
    fn valid_bgra_accepts_correct_size(w in positive_dim(), h in positive_dim()) {
        let data = vec![0u8; (w * h * 4) as usize];
        prop_assert!(validate_bgra(&data, w, h).is_ok());
    }

    #[test]
    fn bgra_rejects_wrong_size(w in positive_dim(), h in positive_dim()) {
        let data = vec![0u8; (w * h * 4) as usize + 1]; // 1 バイト多い
        prop_assert!(validate_bgra(&data, w, h).is_err());
    }

    #[test]
    fn bgra_and_rgba_agree(w in positive_dim(), h in positive_dim()) {
        let correct = vec![0u8; (w * h * 4) as usize];
        let wrong = vec![0u8; (w * h * 4) as usize + 1];
        // 正しいサイズでは両方 Ok
        prop_assert!(validate_rgba(&correct, w, h).is_ok());
        prop_assert!(validate_bgra(&correct, w, h).is_ok());
        // 不正なサイズでは両方 Err
        prop_assert!(validate_rgba(&wrong, w, h).is_err());
        prop_assert!(validate_bgra(&wrong, w, h).is_err());
    }
}
