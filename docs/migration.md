# Native migration

The former Python application remains available in Git at `9e96e5e`.
The native application replaces active Python packaging.

| Existing behavior | Native implementation and verification |
| --- | --- |
| Mute confirmation | Device mirror policy; `device_mirror` and `device_workers` tests |
| Physical gain synchronization | Device mirror conversion; Task 3 adds original Wave XLR limits and synchronization |
| Default input restoration | Owned audio graph and capture bindings; `audio_pins`, `routing`, and `recovery` tests |
| Capture ordering | Identity-bound capture workers; `audio_pins` and `meter_lifecycle` tests |
| Capture byte-flow monitoring | Meter readiness and recovery; `meter_lifecycle` and `recovery` tests |

Automatic disruptive recovery requires an explicit setting.
Normal startup must not restart the audio server or cycle unrelated device profiles.
Keep legacy configuration intact until validated import succeeds.

## Optional state import

Run `cadiswave-maintenance import-legacy --source "$HOME/.config/openwave" --destination "$HOME/.config/cadiswave"`.
The importer validates known JSON schemas and retains source files.
Existing destination files remain unchanged.
Linked ancestry and changed source files prevent import.

## Legacy ownership

CadisWave checks both application bus names before opening vendor controls.
It retains the legacy native vendor lease during device access.
An active Python `openwave.service` prevents vendor access.
Stop that service only after preparing a verified CadisWave installation.
Check `openwave-tray.service` as well.
Its `Wants=openwave.service` dependency can start the disabled legacy service at login.
After installation verification, disable and stop both legacy units:

```sh
systemctl --user disable --now openwave-tray.service openwave.service
systemctl --user is-enabled openwave-tray.service openwave.service
systemctl --user is-active openwave-tray.service openwave.service
```

Both units must report `disabled` and `inactive`.
These status commands return a nonzero exit status for disabled or inactive units.
Keep their unit files and saved configuration for rollback.
Keep `filter-chain.service`, PipeWire, and WirePlumber running.
If vendor control is busy, CadisWave shows the cause and a Retry button.
After resolving the conflict, retry activation without running host setup again.
The native capture service owns capture streams.
The desktop runtime owns hardware synchronization while the application runs, including its tray state.

## Safe activation

Build and stage the native binaries first.
Run the isolated installed smoke check.
Save the legacy service, configuration, and source revision outside the repository.
Stop the legacy service before starting the new desktop runtime.
Keep `filter-chain.service` and unrelated audio services running.
Verify the exact Wave capture, supported control readback, and default input.
Restore the previous control values after acceptance tests.

Use desktop autostart when hardware synchronization must continue after login.
Keep the capture daemon separate from the desktop USB owner.
Legacy mute-sync environment variables have no direct native equivalent.
Use identity-bound source mute synchronization in the desktop runtime.

For rollback, stop CadisWave before restarting the saved legacy service.
Restore the previous default input if activation changed it.
Use the saved legacy source checkout when its service uses Python modules.
Do not launch the retired Python service from the native source directory.

The importer copies settings only.
It does not authorize removal of another installation.
Use the legacy installation's own verified removal procedure when retiring its files.

## Existing USB permissions

CadisWave can retain exact legacy rules for the currently discovered supported devices.
Partial legacy rules do not establish permission for absent or different models.
New devices still require valid kernel permissions and confirmed vendor reads.
CadisWave does not remove the legacy rule during this admission check.

For a verified installation, configure the owned user integration with the example helper:

```bash
cargo build --release --locked -p cadiswave-runtime --examples
./target/release/examples/user-session-setup --configure-user-session "$HOME/.local/bin/cadiswave" --start-at-login
```

The optional login flag enables the managed desktop launcher.
The capture service and desktop launcher use the supplied installation.
Use `Ctrl+Q` or the Quit action for a complete desktop worker drain.

## Processed capture mute

Keep the existing processed default input and its audio service running.
Add that capture node as a device source in Mixer.
Open its source editor and enable Follow hardware mute for the selected Wave device.
Save the source before testing mute.

The source stores the USB descriptor serial in `extra.hardware_mute_serial`.
The runtime requires one connected device and one live capture node for this mapping.
Duplicate identities and unknown graph state cannot authorize mapped mute operations.
Hardware observations also update the processed capture mute.
The mapping does not assign device metadata to the processed node.
Gain and meter identity still require the physical capture device.
