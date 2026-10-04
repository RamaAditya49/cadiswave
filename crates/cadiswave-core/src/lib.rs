pub mod calibration;
pub mod capabilities;
pub mod device_settings;
pub mod effects;
pub mod health;
pub mod locale;
pub mod mic_test;
pub mod model;
pub mod pcm;
pub mod profiles;
pub mod protocol;
pub mod routing;
pub mod scenes;
pub mod voice_presets;

pub const VERSION: &str = env!("CADISWAVE_VERSION");
pub const RUST_COMPILER: &str = env!("CADISWAVE_BUILD_RUSTC");
pub const BUILD_TARGET: &str = env!("CADISWAVE_BUILD_TARGET");

pub mod device_meter;

pub mod pipewire_dump;
