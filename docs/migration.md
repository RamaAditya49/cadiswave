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
