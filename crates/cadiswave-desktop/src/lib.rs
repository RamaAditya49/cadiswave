pub mod actions;
pub mod app;
mod calibration;
pub mod i18n;
pub mod icons;
pub mod tray;
pub mod ui;

pub type Submit = std::rc::Rc<dyn Fn(cadiswave_runtime::controller::AppCommand)>;

pub use cadiswave_core::VERSION;
