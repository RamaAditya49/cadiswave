import unittest
from types import SimpleNamespace
from unittest.mock import Mock, patch

from wavexlr.app import WaveXLRWindow
from wavexlr.tray import TrayIcon


class ControllerTests(unittest.TestCase):
    def test_polling_is_single_flight(self):
        window = SimpleNamespace(
            xlr=Mock(connected=True, get_all=Mock()),
            _poll_busy=False,
            _read_state=Mock(),
            _usb_async=Mock(),
            _on_poll_result=Mock(),
            _on_poll_error=Mock(),
        )

        self.assertTrue(WaveXLRWindow._poll_tick(window))
        self.assertTrue(WaveXLRWindow._poll_tick(window))
        window._usb_async.assert_called_once()

    def test_usb_error_releases_device_and_schedules_reconnect(self):
        window = SimpleNamespace(
            status_label=Mock(),
            xlr=Mock(),
            _stop_polling=Mock(),
            _schedule_reconnect=Mock(),
        )

        WaveXLRWindow._on_usb_error(window, RuntimeError("USB gone"))

        window.xlr.disconnect.assert_called_once_with()
        window._stop_polling.assert_called_once_with()
        window._schedule_reconnect.assert_called_once_with()

    def test_hardware_mute_is_synced_to_linux(self):
        state = {"mute": True}
        window = SimpleNamespace(
            xlr=Mock(get_all=Mock(return_value=state)),
            _linux_mute=None,
        )

        with patch("wavexlr.app.set_source_mute", return_value=True) as set_mute:
            self.assertIs(WaveXLRWindow._read_state(window), state)

        set_mute.assert_called_once_with(True)
        self.assertTrue(window._linux_mute)

    def test_tray_icon_reflects_mute(self):
        tray = TrayIcon()
        tray._build_menu_items()
        tray.set_muted(True)

        self.assertEqual(
            tray._on_item_get_property(None, None, None, None, "IconName").unpack(),
            "microphone-sensitivity-muted-symbolic",
        )
        self.assertEqual(tray._menu_items[2]["label"].unpack(), "Unmute Mic")


if __name__ == "__main__":
    unittest.main()
