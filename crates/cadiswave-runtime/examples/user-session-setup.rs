//! Configure owned user integration after installation verification.
use cadiswave_core::model::{OperationError, Result, default_mixes};
use cadiswave_runtime::{
    paths::{Lease, RuntimePaths},
    setup,
};
fn run() -> Result<()> {
    cadiswave_runtime::process::require_user()?;
    if std::env::args().nth(1).as_deref() != Some("--configure-user-session") {
        return Err(OperationError::invalid(
            "Use --configure-user-session to configure owned audio rules and the capture service",
        ));
    }
    let executable = std::env::current_exe()?;
    let binary = executable
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or_else(|| OperationError::unavailable("Cargo example layout is unavailable"))?
        .join("cadiswave");
    let binary = std::env::args_os()
        .nth(2)
        .map(std::path::PathBuf::from)
        .unwrap_or(binary);
    let paths = RuntimePaths::for_executable(&binary)?;
    let _installation = Lease::installation_shared(&paths.identity)?;
    let outcome = setup::run(&paths, &default_mixes())?;
    println!("{}", outcome.message);
    if outcome.needs_replug {
        return Err(OperationError::unavailable(
            "Reconnect the device before activation",
        ));
    }
    if std::env::args().any(|arg| arg == "--start-at-login") {
        cadiswave_runtime::desktop::set_autostart(&paths, true, true)?;
    }
    Ok(())
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
