pub mod actions;
pub mod app;
pub mod icons;
pub mod tray;
pub mod ui;

pub type Submit = std::rc::Rc<dyn Fn(cadiswave_runtime::controller::AppCommand)>;

pub use cadiswave_core::VERSION;
