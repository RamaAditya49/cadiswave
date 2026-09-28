"""Headless daemon that just runs the audio capture fix."""

import logging
import os
import signal
import time
import sys
import threading

from .audio import AudioManager, set_source_mute
from .device import WaveXLR

logging.basicConfig(level=logging.INFO, format="%(name)s: %(message)s")
log = logging.getLogger("openwave.daemon")


class DeviceSyncManager:
    """Polls Wave XLR hardware mute while the GUI is not running."""

    def __init__(
        self,
        interval=1.0,
        full_sync=False,
        sync_initial=None,
        mute_confirmations=2,
    ):
        self._interval = interval
        self._full_sync = full_sync
        self._sync_initial = full_sync if sync_initial is None else sync_initial
        self._mute_confirmations = max(1, int(mute_confirmations))
        self._running = False
        self._thread = None
        self._xlr = WaveXLR()
        self._present = False
        self._last_state = None
        self._pending_mute = None
        self._pending_count = 0

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
        self._pending_mute = None
        self._pending_count = 0

    def _release_handle(self):
        try:
            self._xlr.disconnect(reset_state=False)
        except Exception:
            pass

    def _connect(self):
        if not self._xlr.connected:
            self._xlr.connect()
            if not self._present:
                sync_type = "control" if self._full_sync else "mute"
                log.info(f"Hardware {sync_type} sync connected")
            self._present = True

    def _sync_mute(self, muted):
        last = self._last_state or {}
        pipewire_synced = last.get("pipewire_synced", False)

        if self._last_state is None and not self._sync_initial:
            self._pending_mute = None
            self._pending_count = 0
            mute_state = "muted" if muted else "unmuted"
            log.info(f"Hardware mute baseline: {mute_state}")
            return muted, True

        if self._last_state is None and self._sync_initial:
            pipewire_synced = set_source_mute(muted)
            mute_state = "muted" if muted else "unmuted"
            if pipewire_synced:
                log.info(f"Hardware mute synced: {mute_state}")
            else:
                log.warning(f"Failed to sync hardware mute: {mute_state}")
            return muted, pipewire_synced

        if last.get("mute") != muted:
            if self._pending_mute != muted:
                self._pending_mute = muted
                self._pending_count = 1
            else:
                self._pending_count += 1

            if self._pending_count < self._mute_confirmations:
                return last.get("mute"), pipewire_synced
        else:
            self._pending_mute = None
            self._pending_count = 0

        should_sync = last.get("mute") != muted or not pipewire_synced

        if should_sync:
            pipewire_synced = set_source_mute(muted)
            mute_state = "muted" if muted else "unmuted"
            if pipewire_synced:
                log.info(f"Hardware mute synced: {mute_state}")
            else:
                log.warning(f"Failed to sync hardware mute: {mute_state}")

        if pipewire_synced:
            self._pending_mute = None
            self._pending_count = 0

        return muted, pipewire_synced

    def _sync_current_state(self):
        if not self._full_sync:
            muted = self._xlr.get_mute()
            confirmed_mute, pipewire_synced = self._sync_mute(muted)
            self._last_state = {
                "mute": confirmed_mute,
                "pipewire_synced": pipewire_synced,
            }
            return

        state = self._xlr.sync_hardware_state()
        last = self._last_state
        confirmed_mute, pipewire_synced = self._sync_mute(state["mute"])
        state["mute"] = confirmed_mute
        state["pipewire_synced"] = pipewire_synced

        if last is None or state["hp_volume_db"] != last["hp_volume_db"]:
            log.info(f"Hardware headphone volume synced: {state['hp_volume_db']:.1f} dB")

        if last is not None and state["gain_raw"] != last["gain_raw"]:
            log.info(f"Hardware gain changed: 0x{state['gain_raw']:04X}")

        self._last_state = state

    def _run(self):
        while self._running:
            try:
                self._connect()
                self._sync_current_state()
                self._release_handle()
                time.sleep(self._interval)
            except Exception as e:
                if self._present:
                    log.warning(f"Hardware sync disconnected: {e}")
                self._disconnect()
                time.sleep(2)


def _env_enabled(name, default=False):
    value = os.environ.get(name)
    if value is None:
        return default
    return value.lower() not in ("", "0", "false", "no", "off")


def _sync_interval():
    try:
        return max(0.5, float(os.environ.get("OPENWAVE_SYNC_INTERVAL", "1.0")))
    except ValueError:
        return 1.0


def _env_int(name, default, minimum=1):
    try:
        return max(minimum, int(os.environ.get(name, str(default))))
    except ValueError:
        return default


def main():
    log.info("Starting OpenWave audio daemon")
    enable_hardware_sync = os.environ.get("OPENWAVE_ENABLE_HARDWARE_SYNC") == "1"
    enable_mute_sync = enable_hardware_sync or _env_enabled("OPENWAVE_ENABLE_MUTE_SYNC", True)
    sync_initial = _env_enabled("OPENWAVE_SYNC_INITIAL_MUTE", enable_hardware_sync)
    mute_confirmations = _env_int("OPENWAVE_MUTE_CONFIRMATIONS", 2)

    def on_status(present, healthy):
        if not present:
            log.info("Device not detected")
        elif healthy:
            log.info("Capture keepalive active")
        else:
            log.warning("Establishing capture keepalive...")

    mgr = AudioManager(on_status_change=on_status)
    sync_mgr = (
        DeviceSyncManager(
            interval=_sync_interval(),
            full_sync=enable_hardware_sync,
            sync_initial=sync_initial,
            mute_confirmations=mute_confirmations,
        )
        if enable_mute_sync
        else None
    )
    mgr.start()
    if sync_mgr:
        sync_mgr.start()
        if enable_hardware_sync:
            log.info("Full hardware control sync enabled")
        else:
            log.info("Hardware mute sync enabled")
    else:
        log.info("Hardware sync disabled")

    def shutdown(sig, frame):
        log.info("Shutting down")
        if sync_mgr:
            sync_mgr.stop()
        mgr.stop()
        sys.exit(0)

    signal.signal(signal.SIGTERM, shutdown)
    signal.signal(signal.SIGINT, shutdown)

    # Keep main thread alive
    while True:
        time.sleep(3600)
