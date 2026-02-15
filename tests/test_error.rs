use raw_player::Error;

#[test]
fn error_is_std_error() {
    let err: Box<dyn std::error::Error> = Box::new(Error::Sdl("test".to_string()));
    assert_eq!(err.to_string(), "SDL error: test");
}
