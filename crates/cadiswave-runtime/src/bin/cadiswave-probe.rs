fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(cadiswave_runtime::probe::cli() as u8)
}
