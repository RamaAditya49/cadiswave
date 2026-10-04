fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(cadiswave_runtime::diag::cli() as u8)
}
