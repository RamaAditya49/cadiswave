import unittest
from unittest.mock import patch

from wavexlr import daemon


class DeviceSyncManagerTests(unittest.TestCase):
    def test_mute_only_sync_skips_initial_state_by_default(self):
        with patch("wavexlr.daemon.WaveXLR") as wave_xlr:
            manager = daemon.DeviceSyncManager(full_sync=False)
            wave_xlr.return_value.get_mute.return_value = True

            with patch("wavexlr.daemon.set_source_mute", return_value=True) as set_mute:
                manager._sync_current_state()

        wave_xlr.return_value.get_mute.assert_called_once_with()
        wave_xlr.return_value.sync_hardware_state.assert_not_called()
        set_mute.assert_not_called()
        self.assertEqual(manager._last_state, {"mute": True, "pipewire_synced": True})

    def test_initial_mute_sync_can_be_explicitly_enabled(self):
        with patch("wavexlr.daemon.WaveXLR") as wave_xlr:
            manager = daemon.DeviceSyncManager(full_sync=False, sync_initial=True)
            wave_xlr.return_value.get_mute.return_value = True

            with patch("wavexlr.daemon.set_source_mute", return_value=True) as set_mute:
                manager._sync_current_state()

        set_mute.assert_called_once_with(True)
        self.assertEqual(manager._last_state, {"mute": True, "pipewire_synced": True})

    def test_mute_change_requires_confirmed_samples(self):
        with patch("wavexlr.daemon.WaveXLR") as wave_xlr:
            manager = daemon.DeviceSyncManager(
                full_sync=False,
                mute_confirmations=2,
            )
            wave_xlr.return_value.get_mute.side_effect = [False, True, True]

            with patch("wavexlr.daemon.set_source_mute", return_value=True) as set_mute:
                manager._sync_current_state()
                manager._sync_current_state()
                set_mute.assert_not_called()

                manager._sync_current_state()

        set_mute.assert_called_once_with(True)
        self.assertEqual(manager._last_state, {"mute": True, "pipewire_synced": True})

    def test_full_sync_still_uses_hardware_state_when_explicitly_enabled(self):
        state = {
            "mute": False,
            "hp_volume_db": -12.0,
            "gain_raw": 0x1200,
        }

        with patch("wavexlr.daemon.WaveXLR") as wave_xlr:
            manager = daemon.DeviceSyncManager(full_sync=True)
            wave_xlr.return_value.sync_hardware_state.return_value = state

            with patch("wavexlr.daemon.set_source_mute", return_value=True) as set_mute:
                manager._sync_current_state()

        wave_xlr.return_value.sync_hardware_state.assert_called_once_with()
        wave_xlr.return_value.get_mute.assert_not_called()
        set_mute.assert_called_once_with(False)
        self.assertTrue(manager._last_state["pipewire_synced"])


class EnvTests(unittest.TestCase):
    def test_env_enabled_defaults_and_false_values(self):
        with patch.dict("wavexlr.daemon.os.environ", {}, clear=True):
            self.assertTrue(daemon._env_enabled("OPENWAVE_ENABLE_MUTE_SYNC", True))

        for value in ("", "0", "false", "no", "off"):
            with patch.dict(
                "wavexlr.daemon.os.environ",
                {"OPENWAVE_ENABLE_MUTE_SYNC": value},
                clear=True,
            ):
                self.assertFalse(daemon._env_enabled("OPENWAVE_ENABLE_MUTE_SYNC", True))


if __name__ == "__main__":
    unittest.main()
