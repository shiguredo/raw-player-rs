//! SDL3 の FFI バインディング (build.rs が生成した bindings を含む)。

#![expect(non_upper_case_globals)]
#![expect(non_camel_case_types)]
#![expect(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/metadata.rs"));
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

unsafe extern "C" {
    // SDL の静的ライブラリにあるが、配布済みの prebuilt バインディングには含まれていない。
    pub(crate) fn SDL_SetWindowAspectRatio(
        window: *mut SDL_Window,
        min_aspect: f32,
        max_aspect: f32,
    ) -> bool;
}
