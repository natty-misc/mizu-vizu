use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(unix)] {
        mod audio_linux;
        pub use self::audio_linux::*;
    } else if #[cfg(windows)] {
        mod audio_windows;
        pub use self::audio_windows::*;
    } else {
        compile_error!("Unsupported platform!");
    }
}

pub const SAMPLES_LOW: usize = 49;

pub const CHANNELS: usize = 2;
pub const DOWNSAMPLE: usize = 15;

pub const SAMPLES: usize = SAMPLES_LOW * DOWNSAMPLE;
pub const WINDOW: usize = SAMPLES * CHANNELS;
