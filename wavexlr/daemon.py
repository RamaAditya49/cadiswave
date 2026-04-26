"""Headless daemon that just runs the audio capture fix."""

import logging
import signal
import time
import sys
import threading

from .audio import AudioManager, set_source_mute
from .device import WaveXLR

logging.basicConfig(level=logging.INFO, format="%(name)s: %(message)s")
log = logging.getLogger("openwave.daemon")


class DeviceSyncManager:
    """Polls Wave XLR hardware controls while the GUI is not running."""

    def __init__(self, interval=0.1):
        self._interval = interval
        self._running = False
        self._thread = None
        self._xlr = WaveXLR()
        self._present = False
        self._last_state = None

    def start(self):
        if self._running:
            return
        self._running = True
        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()

    def stop(self):
        self._running = False
        if self._thread:
            self._thread.join(timeout=3)
        self._disconnect()

    def _disconnect(self):
        try:
            self._xlr.disconnect()
        except Exception:
            pass
        self._present = False
        self._last_state = None

    def _release_handle(self):
        try:
            self._xlr.disconnect(reset_state=False)
        except Exception:
            pass

    def _connect(self):
        if not self._xlr.connected:
            self._xlr.connect()
            if not self._present:
                log.info("Hardware control sync connected")
            self._present = True

    def _run(self):
        while self._running:
            try:
                self._connect()
                state = self._xlr.sync_hardware_state()
                last = self._last_state

                if last is None or state["mute"] != last["mute"]:
                    set_source_mute(state["mute"])
                    mute_state = "muted" if state["mute"] else "unmuted"
                    log.info(f"Hardware mute synced: {mute_state}")

                if last is None or state["hp_volume_db"] != last["hp_volume_db"]:
                    log.info(f"Hardware headphone volume synced: {state['hp_volume_db']:.1f} dB")

                if last is not None and state["gain_raw"] != last["gain_raw"]:
                    log.info(f"Hardware gain changed: 0x{state['gain_raw']:04X}")

                self._last_state = state
                self._release_handle()
                time.sleep(self._interval)
            except Exception as e:
                if self._present:
                    log.warning(f"Hardware control sync disconnected: {e}")
                self._disconnect()
                time.sleep(2)


def main():
    log.info("Starting OpenWave audio daemon")

    def on_status(present, healthy):
        if not present:
            log.info("Device not detected")
        elif healthy:
            log.info("Capture keepalive active")
        else:
            log.warning("Establishing capture keepalive...")

    mgr = AudioManager(on_status_change=on_status)
    sync_mgr = DeviceSyncManager()
    mgr.start()
    sync_mgr.start()

    def shutdown(sig, frame):
        log.info("Shutting down")
        sync_mgr.stop()
        mgr.stop()
        sys.exit(0)

    signal.signal(signal.SIGTERM, shutdown)
    signal.signal(signal.SIGINT, shutdown)

    # Keep main thread alive
    while True:
        time.sleep(3600)
