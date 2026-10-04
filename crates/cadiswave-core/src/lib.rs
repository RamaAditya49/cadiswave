pub mod calibration;
pub mod effects;
pub mod health;
pub mod model;
pub mod profiles;
pub mod protocol;
pub mod routing;
pub mod scenes;

pub const VERSION: &str = env!("CADISWAVE_VERSION");
pub const RUST_COMPILER: &str = env!("CADISWAVE_BUILD_RUSTC");
pub const BUILD_TARGET: &str = env!("CADISWAVE_BUILD_TARGET");
