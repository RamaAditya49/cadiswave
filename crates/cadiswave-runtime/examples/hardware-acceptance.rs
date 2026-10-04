//! Verify reversible Wave XLR controls with one vendor owner.
use cadiswave_core::{model::*, profiles::ProfileId, protocol::DeviceState};
use cadiswave_runtime::{
    device::{DeviceEvent, DeviceManager, VendorDevice},
    paths::Lease,
};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
fn completion(events: &mpsc::Receiver<DeviceEvent>, unit: UnitId, job: u64) -> Result<DeviceState> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match events.recv_timeout(Duration::from_millis(100)) {
            Ok(DeviceEvent::Completed {
                unit: actual,
                job: actual_job,
                result,
            }) if actual == unit && actual_job == job => return result,
            Ok(DeviceEvent::Retired(actual)) if actual == unit => {
                return Err(OperationError::unavailable(
                    "Device disconnected during acceptance",
                ));
            }
            _ => {}
        }
    }
    Err(OperationError::unavailable("Device confirmation deadline"))
}
fn run() -> Result<()> {
    if std::env::args().nth(1).as_deref() != Some("--verify-and-restore") {
        return Err(OperationError::invalid(
            "Use --verify-and-restore to permit reversible control tests",
        ));
    }
    cadiswave_runtime::process::require_user()?;
    let executable = std::env::current_exe()?;
    let binary = executable
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or_else(|| OperationError::unavailable("Cargo example layout is unavailable"))?
        .join("cadiswave");
    let paths = cadiswave_runtime::paths::RuntimePaths::for_executable(&binary)?;
    let _installation = Lease::installation_shared(&paths.identity)?;
    let _lease = Lease::vendor_control(None)?;
    let units: Vec<_> = VendorDevice::scan()?
        .into_iter()
        .filter(|(profile, _, _)| *profile == ProfileId::WaveXlr)
        .collect();
    if units.len() != 1 {
        return Err(OperationError::unavailable(
            "Acceptance requires exactly one original Wave XLR",
        ));
    }
    let (_, bus, address) = units[0];
    let original = {
        let mut vendor = VendorDevice::open(UnitId {
            profile: ProfileId::WaveXlr,
            bus,
            address,
            incarnation: 1,
        })?;
        vendor.read_config()?.state()
    };
    let (mut manager, events) = DeviceManager::start()?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut selected = None;
    while Instant::now() < deadline {
        if let Ok(DeviceEvent::Connected(unit)) = events.recv_timeout(Duration::from_millis(100))
            && unit.id.profile == ProfileId::WaveXlr
            && unit.id.bus == bus
            && unit.id.address == address
        {
            selected = Some(unit.id);
            break;
        }
    }
    let unit = selected
        .ok_or_else(|| OperationError::unavailable("Device worker did not confirm connection"))?;
    let restore = vec![
        DeviceSetting::GainRaw(original.gain_raw),
        DeviceSetting::Mute(original.muted),
        DeviceSetting::HeadphoneDb(original.hp_volume_db),
        DeviceSetting::LowImpedance(
            original
                .low_impedance
                .ok_or_else(|| OperationError::unavailable("Low impedance state is unknown"))?,
        ),
    ];
    let checks = [
        DeviceSetting::GainRaw(if original.gain_raw >= 128 {
            original.gain_raw - 128
        } else {
            128
        }),
        DeviceSetting::Mute(!original.muted),
        DeviceSetting::HeadphoneDb(if original.hp_volume_db <= -127.5 {
            original.hp_volume_db + 0.5
        } else {
            original.hp_volume_db - 0.5
        }),
        DeviceSetting::LowImpedance(!original.low_impedance.unwrap()),
    ];
    let result = (|| {
        for (index, setting) in checks.into_iter().enumerate() {
            let job = (index as u64 + 1) * 2;
            let start = Instant::now();
            manager.submit(unit, job, vec![setting])?;
            let actual = completion(&events, unit, job)?;
            let confirmed = match setting {
                DeviceSetting::GainRaw(v) => actual.gain_raw == v,
                DeviceSetting::Mute(v) => actual.muted == v,
                DeviceSetting::HeadphoneDb(v) => (actual.hp_volume_db - v).abs() < 0.01,
                DeviceSetting::LowImpedance(v) => actual.low_impedance == Some(v),
                _ => false,
            };
            if !confirmed {
                return Err(OperationError::unavailable(
                    "Control readback differs from the requested value",
                ));
            }
            println!(
                "Control {index}: confirmed in {} ms",
                start.elapsed().as_millis()
            );
            manager.submit(unit, job + 1, vec![restore[index]])?;
            completion(&events, unit, job + 1)?;
        }
        Ok(())
    })();
    let restoration = (|| {
        manager.submit(unit, 100, restore)?;
        let state = completion(&events, unit, 100)?;
        if state.gain_raw != original.gain_raw
            || state.muted != original.muted
            || state.hp_volume_db != original.hp_volume_db
            || state.low_impedance != original.low_impedance
            || state.phantom != original.phantom
        {
            return Err(OperationError::unavailable(
                "Original state restoration was not confirmed",
            ));
        }
        Ok(())
    })();
    let stopped = manager.stop();
    restoration?;
    stopped?;
    result?;
    println!(
        "Gain, mute, headphones, and low impedance passed. Original state restored; phantom power retained."
    );
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
