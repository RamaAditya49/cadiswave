# Elgato Wave XLR on Linux: setup, controls, and audio mixing

[CadisWave](https://github.com/RamaAditya49/cadiswave) is free, open-source Wave XLR software for Linux.
It combines microphone controls with a native PipeWire audio mixer.
Rama Aditya (CADIS) develops and maintains the project under the MIT license.

Use this guide to install CadisWave, identify your device, and prepare microphone and application routes.
For an overview, see the [README](../README.md).
For Bahasa Indonesia, see the [Indonesian setup guide](wave-xlr-linux.id.md).

## Does Wave XLR work on Linux?

CadisWave has recorded physical checks for the original Elgato Wave XLR, USB ID `0fd9:007d`.
Gain, mute, headphone volume, and low impedance passed reversible writes with independent USB readback.
Phantom power remained unchanged during physical checks.
Other listed models have protocol mappings but no recorded physical acceptance in this project.
See [hardware support](hardware-support.md) and [test results](verification.md#physical-acceptance).

Linux audio capture and USB vendor controls are separate paths.
A working microphone input does not prove that the application has permission to change hardware settings.

## Install CadisWave

Follow [Build and run](../README.md#build-and-run) for dependencies, the pinned Rust toolchain, and installation commands.
The current native application uses Rust 1.98.1, GTK 4.14+, libadwaita 1.5+, and libusb 1.0.
Audio routing uses PipeWire, pipewire-pulse, WirePlumber, ALSA utilities, and SWH LADSPA plugins.

| System | Evidence or installation route |
| --- | --- |
| Zorin OS 18.1 | [Recorded physical checks](verification.md) with host runtime libraries and an Ubuntu development SDK |
| Ubuntu 24.04 | [CI workflow](../.github/workflows/tests.yml) and [dependency commands](../README.md#build-and-run) |
| Bazzite / Fedora Atomic | [Host installation guide](install-bazzite.md); no recorded physical acceptance on these images |
| Other Linux distributions | Check library versions and package names; compatibility requires verification on your system |

Existing `v0.1.x` GitHub releases contain the earlier OpenWave application.
Use the current source instructions for native CadisWave.
The Flatpak route is experimental; a manifest does not establish a published, validated package.

## Check device identity and USB permissions

1. Connect the Wave XLR to your computer.
2. Read the connected USB identity:

   ```bash
   lsusb -d 0fd9:
   ```

3. Match the product ID against [hardware support](hardware-support.md).
4. Close other applications that own Wave vendor controls.
5. Prepare administrator-managed USB permissions for a user-prefix installation.
6. Start CadisWave as your login user.
7. Confirm the selected device in the dashboard.

The [host integration guide](install-bazzite.md#first-run-host-integration) describes USB rules and user-service setup.
Its USB permission guidance also applies to a native user-prefix installation on mutable distributions.
A trusted system installation can provide the application's privileged setup route.
A helper from your checkout or home directory cannot use that route.

If you used OpenWave, follow the [migration guide](migration.md) before granting CadisWave vendor control.
The capture daemon maintains input streams separately from the desktop's USB owner.

## Adjust gain, mute, and headphones

| Original Wave XLR control | CadisWave scope |
| --- | --- |
| Microphone gain | Adjust gain up to 75 dB |
| Mute | Read and change the hardware mute state |
| Headphone volume | Read and change the headphone level |
| Low impedance | Read and change the hardware setting |
| 48 V phantom power | Available hardware control; physical checks did not change this setting |

The dashboard separates confirmed hardware values from pending edits.
Use **M** to toggle mute and the dial slider's arrow keys to adjust its selected value.
Use **Ctrl+R** to reconnect.

Check your microphone manufacturer's instructions before enabling phantom power.
A scene cannot toggle phantom power.

Clipguard control, hardware low-cut, LED changes, device persistence, and sample-rate changes remain unavailable.
Original Wave XLR hardware monitor mix is not mapped.
Software effects operate through PipeWire; they do not change the device's onboard DSP.

## Mix application and microphone audio

CadisWave can manage game, chat, browser, and microphone sources through PipeWire.
Mixes have independent sends and master controls.
Published mix inputs can feed OBS, recording software, or voice applications.

1. Open **Mixer**.
2. Add an application source for an application you want to route.
3. Add a capture source for your microphone.
4. Select the exact microphone capture node in the source editor.
5. Configure sends to your chosen mixes.
6. Check source, send, and mix mute states.
7. Select the desired hardware output for monitoring.
8. Select the published mix input in OBS or your voice application.

Published mix inputs use `cadiswave_capture_<mix_id>` node names.
Removing a mix removes its input.
Select a replacement input in each consuming application after removal.

Enable **Follow hardware mute** in a processed capture source editor to synchronize its mute with the selected Wave device.
Software effects include low-cut, gate, compression, EQ, delay, and mono.
SWH LADSPA plugins must be available for routes that use those plugins.

Keep voice return audio outside its own microphone mix to prevent feedback.
Hardware direct monitoring and software monitoring can produce doubled audio.
Use [routing troubleshooting](troubleshooting.md#application-or-mix-is-silent) to inspect the complete route.

## Is CadisWave a Wave Link alternative for Linux?

Yes, for the supported device controls and PipeWire mixing workflows described above.
CadisWave is an independent application derived from OpenWave, with its original MIT notices retained.
It is not an Elgato product and does not provide every Wave Link feature.

Elgato's [Wave Link 3 setup guide](https://help.elgato.com/hc/en-us/articles/46941178829585-Wave-Link-3-0-Software-Initial-Setup) lists Windows, Mac, and Windows on Arm.
It does not list Linux.
This operating-system statement was checked on 2026-10-04.

## Get help

Create a [CadisWave issue](https://github.com/RamaAditya49/cadiswave/issues) with the symptom and reproduction steps.
Include the device model, USB ID, Linux distribution, and relevant audio-library versions.
Use [privacy-reduced diagnostics](troubleshooting.md#diagnostics-and-privacy) when additional evidence is needed.
Review diagnostic files before sharing them.

See the [README FAQ](../README.md#frequently-asked-questions) for concise answers.
See [verification](verification.md) for measured results and physical test limits.
