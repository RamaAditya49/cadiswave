import subprocess
import unittest
from unittest.mock import patch

from wavexlr import audio


class PWDumpTests(unittest.TestCase):
    def test_source_lookup_tolerates_invalid_utf8_from_pipewire_props(self):
        source_name = (
            "alsa_input.usb-Elgato_Systems_Elgato_Wave_XLR_"
            "A01DA41220BF59-00.pro-input-0"
        )
        raw_dump = (
            b'[{"id":80,"type":"PipeWire:Interface:Node","info":{"props":{'
            b'"node.name":"' + source_name.encode() + b'",'
            b'"media.class":"Audio/Source",'
            b'"audio.position":"\xe0"'
            b"}}}]"
        )

        completed = subprocess.CompletedProcess(
            ["pw-dump", "--no-colors"], 0, stdout=raw_dump, stderr=b""
        )

        with patch("wavexlr.audio.subprocess.run", return_value=completed):
            self.assertEqual(
                audio._get_source_node(),
                {"id": "80", "name": source_name},
            )

    def test_source_lookup_tolerates_malformed_audio_position_prop(self):
        source_name = (
            "alsa_input.usb-Elgato_Systems_Elgato_Wave_XLR_"
            "A01DA41220BF59-00.pro-input-0"
        )
        raw_dump = f"""
[
  {{
    "id": 80,
    "type": "PipeWire:Interface:Node",
    "info": {{
      "props": {{
        "node.name": "{source_name}",
        "audio.position": "\\u000e"=}}XDR[,
        "media.class": "Audio/Source"
      }}
    }}
  }}
]
""".encode()

        completed = subprocess.CompletedProcess(
            ["pw-dump", "--no-colors"], 0, stdout=raw_dump, stderr=b""
        )

        with patch("wavexlr.audio.subprocess.run", return_value=completed):
            self.assertEqual(
                audio._get_source_node(),
                {"id": "80", "name": source_name},
            )

    def test_ensure_source_profile_enables_wave_xlr_mono_input_when_off(self):
        wave_device = {
            "id": 53,
            "type": "PipeWire:Interface:Device",
            "info": {
                "props": {
                    "device.name": (
                        "alsa_card.usb-Elgato_Systems_Elgato_Wave_XLR_"
                        "A01DA41220BF59-00"
                    ),
                    "device.product.name": "Elgato Wave XLR",
                    "media.class": "Audio/Device",
                },
                "params": {
                    "EnumProfile": [
                        {"index": 0, "name": "off"},
                        {"index": 1, "name": "output:analog-stereo+input:mono-fallback"},
                    ],
                    "Profile": [{"index": 0, "name": "off"}],
                },
            },
        }

        completed = subprocess.CompletedProcess(["wpctl"], 0, stdout="", stderr="")

        with patch("wavexlr.audio._pw_dump", return_value=[wave_device]), patch(
            "wavexlr.audio.subprocess.run", return_value=completed
        ) as run:
            self.assertTrue(audio._ensure_source_profile())

        run.assert_called_once_with(
            ["wpctl", "set-profile", "53", "1"],
            capture_output=True,
            text=True,
            timeout=5,
        )

    def test_set_source_mute_updates_physical_and_processed_sources(self):
        completed = subprocess.CompletedProcess(["wpctl"], 0, stdout="", stderr="")

        with patch(
            "wavexlr.audio._get_source_node",
            return_value={"id": "80", "name": "alsa_input.wave"},
        ), patch(
            "wavexlr.audio._get_processed_source_node",
            return_value={"id": "91", "name": "effect_output.wave_xlr_voice_bass"},
        ), patch(
            "wavexlr.audio.subprocess.run", return_value=completed
        ) as run:
            self.assertTrue(audio.set_source_mute(True))

        self.assertEqual(run.call_count, 2)
        run.assert_any_call(
            ["wpctl", "set-mute", "80", "1"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        run.assert_any_call(
            ["wpctl", "set-mute", "91", "1"],
            capture_output=True,
            text=True,
            timeout=5,
        )

    def test_audio_manager_waits_for_processed_source_during_startup_grace(self):
        manager = audio.AudioManager(processed_source_grace=10)
        manager._started_at = 100.0
        source = {"id": "80", "name": "alsa_input.wave"}

        self.assertIsNone(manager._select_default_source(source, None, now=105.0))

    def test_audio_manager_falls_back_to_raw_source_after_grace(self):
        manager = audio.AudioManager(processed_source_grace=10)
        manager._started_at = 100.0
        source = {"id": "80", "name": "alsa_input.wave"}

        self.assertEqual(
            manager._select_default_source(source, None, now=111.0),
            source,
        )

    def test_audio_manager_prefers_processed_source_when_available(self):
        manager = audio.AudioManager(processed_source_grace=10)
        source = {"id": "80", "name": "alsa_input.wave"}
        processed_source = {
            "id": "91",
            "name": "effect_output.wave_xlr_voice_bass",
        }

        self.assertEqual(
            manager._select_default_source(source, processed_source, now=101.0),
            processed_source,
        )

    def test_audio_manager_detects_capture_without_data(self):
        manager = audio.AudioManager()
        manager._stall_timeout = 5
        manager._last_data_at = 100.0

        with patch.object(manager, "_cat_alive", return_value=True):
            self.assertFalse(manager._capture_stalled(now=104.0))
            self.assertTrue(manager._capture_stalled(now=106.0))

        with patch.object(manager, "_cat_alive", return_value=False):
            self.assertFalse(manager._capture_stalled(now=106.0))


if __name__ == "__main__":
    unittest.main()
