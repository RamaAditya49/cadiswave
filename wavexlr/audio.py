"""
Wave XLR PipeWire audio manager.

Fixes the race condition where mic capture fails if playback starts first.
Strategy: run `pw-cat --record` targeted at the Wave XLR source, piping to
/dev/null. This keeps the ALSA capture stream permanently open.
"""

import json
import re
import subprocess
import threading
import time
import logging

log = logging.getLogger("wavexlr.audio")

SOURCE_MATCH = "alsa_input.usb-Elgato_Systems_Elgato_Wave_XLR"


def _pw_dump():
    """Get PipeWire object dump as JSON."""
    try:
        r = subprocess.run(
            ["pw-dump", "--no-colors"], capture_output=True, text=True, timeout=5
        )
        if r.returncode == 0:
            return json.loads(r.stdout)
    except Exception:
        pass
    return []


def _get_node(match):
    """Get the PipeWire node id and name for the first matching node."""
    for obj in _pw_dump():
        if obj.get("type") != "PipeWire:Interface:Node":
            continue
        props = obj.get("info", {}).get("props", {})
        name = props.get("node.name", "")
        if name.startswith(match):
            return {"id": str(obj.get("id")), "name": name}
    return None


def _get_source_node():
    """Get the PipeWire node id and name of the Wave XLR source."""
    return _get_node(SOURCE_MATCH)


def _get_default_source_name():
    """Read the current PipeWire default audio source name."""
    try:
        r = subprocess.run(
            ["pw-metadata", "-n", "default", "0", "default.audio.source"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        if r.returncode != 0:
            return None

        match = re.search(r"value:'([^']*)'", r.stdout)
        if not match:
            return None

        return json.loads(match.group(1)).get("name")
    except Exception:
        return None


def _set_default_source(node):
    """Make the Wave XLR the default input so apps keep using it after reconnect."""
    if _get_default_source_name() == node["name"]:
        return True

    try:
        r = subprocess.run(
            ["wpctl", "set-default", node["id"]],
            capture_output=True,
            text=True,
            timeout=5,
        )
        if r.returncode == 0:
            log.info("Set Wave XLR as default audio source")
            return True
        log.warning(f"Failed to set default source: {r.stderr.strip()}")
    except Exception as e:
        log.warning(f"Failed to set default source: {e}")
    return False


def set_source_mute(muted):
    """Mirror hardware mute to the PipeWire source mute state."""
    source = _get_source_node()
    if not source:
        return False

    try:
        r = subprocess.run(
            ["wpctl", "set-mute", source["id"], "1" if muted else "0"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        if r.returncode == 0:
            return True
        log.warning(f"Failed to set PipeWire source mute: {r.stderr.strip()}")
    except Exception as e:
        log.warning(f"Failed to set PipeWire source mute: {e}")
    return False


class AudioManager:
    """Keeps the Wave XLR capture stream active via pw-cat subprocess."""

    def __init__(self, on_status_change=None):
        self._running = False
        self._thread = None
        self._cat_proc = None
        self._healthy = False
        self._device_present = False
        self.on_status_change = on_status_change

    @property
    def healthy(self):
        return self._healthy

    @property
    def device_present(self):
        return self._device_present

    def start(self):
        if self._running:
            return
        self._running = True
        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()

    def stop(self):
        self._running = False
        self._kill_cat()
        if self._thread:
            self._thread.join(timeout=3)

    def _kill_cat(self):
        if self._cat_proc and self._cat_proc.poll() is None:
            self._cat_proc.terminate()
            try:
                self._cat_proc.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self._cat_proc.kill()
            log.info("Stopped capture keepalive")
        self._cat_proc = None

    def _start_cat(self, source_name):
        """Start pw-cat to keep capture stream open."""
        self._kill_cat()
        with open("/dev/null", "wb") as devnull:
            self._cat_proc = subprocess.Popen(
                [
                    "pw-cat", "--record",
                    "--target", source_name,
                    "--channels", "1",
                    "--format", "s16",
                    "--rate", "48000",
                    "-",
                ],
                stdout=devnull,
                stderr=subprocess.DEVNULL,
            )
        log.info(f"Started capture keepalive (PID {self._cat_proc.pid})")

    def _cat_alive(self):
        return self._cat_proc is not None and self._cat_proc.poll() is None

    def _update_status(self, present, healthy):
        changed = (present != self._device_present) or (healthy != self._healthy)
        self._device_present = present
        self._healthy = healthy
        if changed and self.on_status_change:
            self.on_status_change(present, healthy)

    def _run(self):
        while self._running:
            try:
                source = _get_source_node()

                if not source:
                    if self._device_present:
                        self._kill_cat()
                    self._update_status(False, False)
                    time.sleep(2)
                    continue

                _set_default_source(source)

                if not self._cat_alive():
                    self._start_cat(source["name"])
                    time.sleep(1)

                healthy = self._cat_alive()
                self._update_status(True, healthy)
                time.sleep(2)

            except Exception as e:
                log.error(f"Audio manager error: {e}")
                self._update_status(self._device_present, False)
                time.sleep(2)
