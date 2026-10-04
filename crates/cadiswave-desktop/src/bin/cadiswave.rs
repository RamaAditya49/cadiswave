//! Informational and removal dispatch deliberately precedes GTK initialization.
use cadiswave_runtime::{desktop, paths::RuntimePaths, process, uninstall};

const HELP: &str = "CadisWave — Wave hardware controls and PipeWire audio mixes

Usage: cadiswave [GUI OPTIONS]
       cadiswave --uninstall [--delete-settings] [--yes] [--dry-run]
       cadiswave --migrate-launchers-from PREVIOUS_EXECUTABLE [--dry-run] [--yes]

  --hide             Start hidden only when a system tray host is available
  --version          Print the installed version without opening audio or USB
  --uninstall        Remove this installation or show package-manager guidance
  --delete-settings  With --uninstall, also delete settings and saved scenes
  --migrate-launchers-from PATH
                     Explicitly migrate proven old native user launchers to this
                     installation's stable profile launcher, before removing PATH
  --yes              Confirm removal or consent to the selected previous executable
  --dry-run          Inspect removal or launcher migration without making changes
  -h, --help         Show this help

Standard GApplication and GTK options remain available on the GUI path.";

fn migration_options(args: &[String]) -> Result<(std::path::PathBuf, bool, bool), &'static str> {
    let mut previous = None;
    let mut yes = false;
    let mut dry_run = false;
    let mut arguments = args.iter().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--migrate-launchers-from" if previous.is_none() => {
                let value = arguments.next().ok_or("Missing PREVIOUS_EXECUTABLE")?;
                if value.starts_with('-') || !std::path::Path::new(value).is_absolute() {
                    return Err(
                        "PREVIOUS_EXECUTABLE must be the exact absolute previous bin/cadiswave path",
                    );
                }
                previous = Some(std::path::PathBuf::from(value));
            }
            "--yes" if !yes => yes = true,
            "--dry-run" if !dry_run => dry_run = true,
            _ => {
                return Err(
                    "Launcher migration cannot be combined with GUI, uninstall, informational, duplicate or unknown options",
                );
            }
        }
    }
    Ok((previous.ok_or("Missing PREVIOUS_EXECUTABLE")?, yes, dry_run))
}

fn main() {
    std::process::exit(dispatch(std::env::args().collect()));
}

fn dispatch(args: Vec<String>) -> i32 {
    let has = |flag: &str| args.iter().skip(1).any(|argument| argument == flag);
    if has("--migrate-launchers-from") {
        let (previous, yes, dry_run) = match migration_options(&args) {
            Ok(options) => options,
            Err(error) => {
                eprintln!("cadiswave: {error}");
                return 2;
            }
        };
        if let Err(error) = process::require_user() {
            eprintln!("cadiswave: {error}");
            return 1;
        }
        return match RuntimePaths::discover() {
            Ok(paths) => desktop::migrate_launchers_cli(&paths, &previous, yes, dry_run),
            Err(error) => {
                eprintln!("cadiswave: {error}");
                1
            }
        };
    }
    let removal = has("--uninstall");
    if removal
        && args.iter().skip(1).any(|argument| {
            !matches!(
                argument.as_str(),
                "--uninstall" | "--delete-settings" | "--yes" | "--dry-run"
            )
        })
    {
        eprintln!(
            "cadiswave: --uninstall cannot be combined with GUI, informational, or unknown options"
        );
        return 2;
    }
    if !removal && (has("--delete-settings") || has("--yes") || has("--dry-run")) {
        eprintln!(
            "cadiswave: --delete-settings requires --uninstall; --yes and --dry-run require --uninstall or --migrate-launchers-from"
        );
        return 2;
    }
    if has("--help") || has("-h") {
        println!("{HELP}");
        return 0;
    }
    if has("--version") {
        println!("cadiswave {}", cadiswave_core::VERSION);
        return 0;
    }
    if let Err(error) = process::require_user() {
        eprintln!("cadiswave: {error}");
        return 1;
    }
    if removal {
        return match RuntimePaths::discover() {
            Ok(paths) => uninstall::cli(
                &paths,
                has("--delete-settings"),
                has("--yes"),
                has("--dry-run"),
            ),
            Err(error) => {
                eprintln!("cadiswave: {error}");
                1
            }
        };
    }
    cadiswave_desktop::app::run(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcher_migration_rejects_mixed_or_ambiguous_dispatch() {
        for suffix in [
            "--hide",
            "--uninstall",
            "--delete-settings",
            "--version",
            "--help",
            "--unknown",
            "--yes --yes",
            "--migrate-launchers-from /other/bin/cadiswave",
        ] {
            let args = format!("cadiswave --migrate-launchers-from /old/bin/cadiswave {suffix}")
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            assert!(migration_options(&args).is_err(), "{suffix}");
        }
        for arguments in [
            "cadiswave --migrate-launchers-from",
            "cadiswave --migrate-launchers-from --yes",
            "cadiswave --migrate-launchers-from relative/bin/cadiswave",
        ] {
            assert!(
                migration_options(
                    &arguments
                        .split_whitespace()
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                )
                .is_err()
            );
        }
    }
}
