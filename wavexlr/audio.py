"""
Wave XLR PipeWire audio manager.

Fixes the race condition where mic capture fails if playback starts first.
Strategy: run `pw-cat --record` targeted at the Wave XLR source, piping to
/dev/null. This keeps the ALSA capture stream permanently open.
"""

import json
import os
import re
import subprocess
import threading
import time
import logging

log = logging.getLogger("wavexlr.audio")

SOURCE_MATCH = "alsa_input.usb-Elgato_Systems_Elgato_Wave_XLR"
PROCESSED_SOURCE_MATCH = "effect_output.wave_xlr_voice_bass"
DEVICE_MATCH = "Elgato Wave XLR"
PROFILE_NAME = "output:analog-stereo+input:mono-fallback"


def _env_float(name, default, minimum=0.0):
    try:
        return max(minimum, float(os.environ.get(name, str(default))))
    except ValueError:
        return default


def _parse_pw_dump(stdout):
    """Parse pw-dump output, tolerating malformed volatile PipeWire props."""
    try:
        return json.loads(stdout)
    except json.JSONDecodeError:
        cleaned = re.sub(
            r'^\s*"audio\.position":.*,\n',
            "",
            stdout,
            flags=re.MULTILINE,
        )
        return json.loads(cleaned)


def _pw_dump():
    """Get PipeWire object dump as JSON."""
    try:
        r = subprocess.run(
            ["pw-dump", "--no-colors"], capture_output=True, timeout=5
        )
        if r.returncode == 0:
            stdout = r.stdout
            if isinstance(stdout, bytes):
                stdout = stdout.decode("utf-8", errors="replace")
            return _parse_pw_dump(stdout)
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


def _get_processed_source_node():
    """Get the preferred processed Wave XLR source when the voice filter is running."""
    return _get_node(PROCESSED_SOURCE_MATCH)


def _is_wave_device(props):
    """Return whether PipeWire device props refer to the Wave XLR card."""
    return (
        props.get("device.product.name") == DEVICE_MATCH
        or props.get("device.description") == DEVICE_MATCH
        or "Elgato_Wave_XLR" in str(props.get("device.name", ""))
    )


def _profile_name(profile):
    profiles = profile if isinstance(profile, list) else []
    if not profiles:
        return None
    return profiles[0].get("name")


def _profile_index(profiles, name):
    for profile in profiles if isinstance(profiles, list) else []:
        if profile.get("name") == name:
            return str(profile.get("index"))
    return None


def _ensure_source_profile():
    """Activate the Wave XLR PipeWire profile with the stable mono capture node."""
    for obj in _pw_dump():
        if obj.get("type") != "PipeWire:Interface:Device":
            continue

        info = obj.get("info", {})
        props = info.get("props", {})
        if not _is_wave_device(props):
            continue

        params = info.get("params", {})
        if _profile_name(params.get("Profile")) == PROFILE_NAME:
            return True

        profile_index = _profile_index(params.get("EnumProfile"), PROFILE_NAME)
        if profile_index is None:
            return False

        try:
            r = subprocess.run(
                ["wpctl", "set-profile", str(obj.get("id")), profile_index],
                capture_output=True,
                text=True,
                timeout=5,
            )
            if r.returncode == 0:
                log.info("Enabled Wave XLR PipeWire mono input profile")
                return True
            log.warning(f"Failed to enable Wave XLR profile: {r.stderr.strip()}")
        except Exception as e:
            log.warning(f"Failed to enable Wave XLR profile: {e}")
        return False

    return False


def _reset_source_profile():
    """Turn the Wave XLR profile off and on to reopen a stalled capture stream."""
    for obj in _pw_dump():
        if obj.get("type") != "PipeWire:Interface:Device":
            continue
        if not _is_wave_device(obj.get("info", {}).get("props", {})):
            continue
        try:
            subprocess.run(
                ["wpctl", "set-profile", str(obj.get("id")), "0"],
                capture_output=True,
                timeout=5,
            )
        except Exception as e:
            log.warning(f"Failed to reset Wave XLR profile: {e}")
            return False
        time.sleep(1)
        return _ensure_source_profile()
    return False


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
    """Make the selected Wave XLR path the default input."""
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
            log.info(f"Set default audio source: {node['name']}")
            return True
        log.warning(f"Failed to set default source: {r.stderr.strip()}")
    except Exception as e:
        log.warning(f"Failed to set default source: {e}")
    return False


def set_source_mute(muted):
    """Mirror hardware mute to the PipeWire source mute state."""
    sources = [_get_source_node(), _get_processed_source_node()]
    sources = [source for source in sources if source]
    if not sources:
        return False

    ok = True
    try:
        for source in sources:
            r = subprocess.run(
                ["wpctl", "set-mute", source["id"], "1" if muted else "0"],
                capture_output=True,
                text=True,
                timeout=5,
            )
            if r.returncode != 0:
                ok = False
                log.warning(f"Failed to set PipeWire source mute: {r.stderr.strip()}")
    except Exception as e:
        log.warning(f"Failed to set PipeWire source mute: {e}")
        return False
    return ok


class AudioManager:
    """Keeps the Wave XLR capture stream active via pw-cat subprocess."""

    def __init__(self, on_status_change=None, processed_source_grace=None):
        self._running = False
        self._thread = None
        self._cat_proc = None
        self._healthy = False
        self._device_present = False
        self.on_status_change = on_status_change
        self._processed_source_grace = (
            _env_float("OPENWAVE_PROCESSED_SOURCE_GRACE", 20.0)
            if processed_source_grace is None
            else processed_source_grace
        )
        self._started_at = time.monotonic()
        self._stall_timeout = _env_float("OPENWAVE_CAPTURE_STALL_TIMEOUT", 5.0, 1.0)
        self._last_data_at = time.monotonic()

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
        self._cat_proc = subprocess.Popen(
            [
                "pw-cat", "--record",
                "--target", source_name,
                "--channels", "1",
                "--format", "s16",
                "--rate", "48000",
                "-",
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        self._last_data_at = time.monotonic()
        threading.Thread(
            target=self._drain_cat, args=(self._cat_proc.stdout,), daemon=True
        ).start()
        log.info(f"Started capture keepalive (PID {self._cat_proc.pid})")

    def _drain_cat(self, stdout):
        """Read and drop captured audio, and record when data last arrived."""
        for _ in iter(lambda: stdout.read(4096), b""):
            self._last_data_at = time.monotonic()

    def _capture_stalled(self, now=None):
        now = time.monotonic() if now is None else now
        return self._cat_alive() and now - self._last_data_at > self._stall_timeout

    def _cat_alive(self):
        return self._cat_proc is not None and self._cat_proc.poll() is None

    def _update_status(self, present, healthy):
        changed = (present != self._device_present) or (healthy != self._healthy)
        self._device_present = present
        self._healthy = healthy
        if changed and self.on_status_change:
            self.on_status_change(present, healthy)

    def _select_default_source(self, source, processed_source, now=None):
        if processed_source:
            return processed_source

        now = time.monotonic() if now is None else now
        if now - self._started_at < self._processed_source_grace:
            return None

        return source

    def _run(self):
        self._started_at = time.monotonic()
        while self._running:
            try:
                source = _get_source_node()

                if not source:
                    _ensure_source_profile()
                    if self._device_present:
                        self._kill_cat()
                    self._update_status(False, False)
                    time.sleep(2)
                    continue

                default_source = self._select_default_source(
                    source,
                    _get_processed_source_node(),
                )
                if default_source:
                    _set_default_source(default_source)

                if not self._cat_alive():
                    self._start_cat(source["name"])
                    time.sleep(1)

                if self._capture_stalled():
                    log.warning("Wave XLR capture stalled, resetting profile")
                    self._kill_cat()
                    _reset_source_profile()
                    self._update_status(True, False)
                    time.sleep(2)
                    continue

                healthy = self._cat_alive()
                self._update_status(True, healthy)
                time.sleep(2)

            except Exception as e:
                log.error(f"Audio manager error: {e}")
                self._update_status(self._device_present, False)
                time.sleep(2)
