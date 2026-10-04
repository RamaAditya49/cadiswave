#!/bin/bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
for capability in bwrap gnome-shell gnome-extensions gjs python3; do
    command -v "$capability" >/dev/null || { printf 'Missing capability: %s\n' "$capability" >&2; exit 1; }
done
proof_dir=$(mktemp -d /tmp/cadiswave-status-gnome.XXXXXXXX)
chmod 700 "$proof_dir"
mkdir -p "$proof_dir/home/.local/share/gnome-shell/extensions/cadiswave-status@cadis.digital" "$proof_dir/home/.local/share/applications" "$proof_dir/run"
chmod 700 "$proof_dir/run"
cp gnome-extension/{extension.js,metadata.json} icons/cadiswave{,-green,-red,-orange}.svg "$proof_dir/home/.local/share/gnome-shell/extensions/cadiswave-status@cadis.digital/"
cat > "$proof_dir/home/.local/share/applications/cadiswave.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=CadisWave
Exec=gjs -m /work/fixture.js
Icon=cadiswave
StartupWMClass=io.github.RamaAditya49.CadisWave
DESKTOP
mkdir -p "$proof_dir/home/.local/share/gnome-shell/extensions/cadiswave-test@cadis.digital"
printf '%s\n' '{"uuid":"cadiswave-test@cadis.digital","name":"Private test helper","description":"Private Shell test only","shell-version":["46"]}' > "$proof_dir/home/.local/share/gnome-shell/extensions/cadiswave-test@cadis.digital/metadata.json"
cat > "$proof_dir/home/.local/share/gnome-shell/extensions/cadiswave-test@cadis.digital/extension.js" <<'HELPER'
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
export default class TestHelper extends Extension {
    enable() { global.context.unsafe_mode = true; }
    disable() { global.context.unsafe_mode = false; }
}
HELPER
cat > "$proof_dir/fixture.js" <<'JS'
import Gtk from 'gi://Gtk?version=4.0';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
const app = new Gtk.Application({application_id: 'io.github.RamaAditya49.CadisWave'});
app.connect('activate', () => {
    const action = Gio.SimpleAction.new_stateful('status-icon', new GLib.VariantType('s'), new GLib.Variant('s', 'cadiswave-green'));
    action.connect('activate', (_, value) => action.set_state(value));
    app.add_action(action);
    const quit = new Gio.SimpleAction({name: 'quit'});
    quit.connect('activate', () => app.quit());
    app.add_action(quit);
    new Gtk.ApplicationWindow({application: app, title: 'Private status fixture', default_width: 320, default_height: 240}).present();
});
app.run([]);
JS
cat > "$proof_dir/check.py" <<'PY'
import time,json,subprocess
from gi.repository import Gio,GLib
bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
def call(dest,path,interface,method,parameters=None):
 return bus.call_sync(dest,path,interface,method,parameters,None,Gio.DBusCallFlags.NO_AUTO_START,4000,None).unpack()
def evaluate(source):
 success,result=call('org.gnome.Shell','/org/gnome/Shell','org.gnome.Shell','Eval',GLib.Variant('(s)',(source,)))
 assert success,result
 return json.loads(result)
def wait(test):
 deadline=time.monotonic()+15
 while time.monotonic()<deadline:
  try:
   value=test()
   if value:return value
  except Exception:pass
  time.sleep(.15)
 raise RuntimeError('Private Shell check did not settle')
wait(lambda:call('org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus','NameHasOwner',GLib.Variant('(s)',('org.gnome.Shell',)))[0])
wait(lambda:evaluate("!!global.testCadisReady || !!imports.gi.Shell.AppSystem.get_default().lookup_app('cadiswave.desktop')"))
subprocess.Popen(['gjs','-m','/work/fixture.js'],stdout=open('/work/fixture.log','w'),stderr=subprocess.STDOUT)
wait(lambda:call('org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus','NameHasOwner',GLib.Variant('(s)',('io.github.RamaAditya49.CadisWave',)))[0])
evaluate("(async()=>{const {AppIcon}=await import('resource:///org/gnome/shell/ui/appDisplay.js');global.testCadisTile=new AppIcon(imports.gi.Shell.AppSystem.get_default().lookup_app('cadiswave.desktop'));const Main=await import('resource:///org/gnome/shell/ui/main.js');Main.layoutManager.uiGroup.add_child(global.testCadisTile);global.testCadisIcon=global.testCadisTile.icon.icon;return true;})()")
evaluate("global.testOtherIcon=imports.gi.Shell.AppSystem.get_default().lookup_app('org.gnome.Nautilus.desktop').create_icon_texture(48);global.testOtherOriginal=global.testOtherIcon.gicon.to_string();true")
def icon():return evaluate('global.testCadisIcon.gicon.to_string()')
def activate(name,value=None):
 call('io.github.RamaAditya49.CadisWave','/io/github/RamaAditya49/CadisWave','org.gtk.Actions','Activate',GLib.Variant('(sava{sv})',(name,[] if value is None else [GLib.Variant('s',value)],{})))
for name in ['cadiswave-green','cadiswave-red','cadiswave-orange','cadiswave-red','cadiswave-green']:
 activate('status-icon',name)
 wait(lambda:name+'.svg' in icon())
 assert evaluate('global.testOtherIcon.gicon.to_string()===global.testOtherOriginal')
 print(name+': actual Shell icon updated; other application retained its icon',flush=True)
activate('status-icon','unexpected-icon');wait(lambda:'.svg' not in icon())
print('Unknown action state retained the original icon',flush=True)
activate('status-icon','cadiswave-orange');wait(lambda:'cadiswave-orange.svg' in icon())
subprocess.run(['gnome-extensions','disable','cadiswave-status@cadis.digital'],check=True)
wait(lambda:'.svg' not in icon())
print('Disable restored the original icon',flush=True)
subprocess.run(['gnome-extensions','enable','cadiswave-status@cadis.digital'],check=True)
wait(lambda:'cadiswave-orange.svg' in icon())
activate('quit')
wait(lambda:'.svg' not in icon())
print('Application exit restored the original icon',flush=True)
subprocess.Popen(['gjs','-m','/work/fixture.js'],stdout=open('/work/restart.log','w'),stderr=subprocess.STDOUT)
wait(lambda:call('org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus','NameHasOwner',GLib.Variant('(s)',('io.github.RamaAditya49.CadisWave',)))[0])
wait(lambda:'cadiswave-green.svg' in icon())
print('Application restart published the new owner state',flush=True)
activate('quit')
wait(lambda:'.svg' not in icon())
PY
cat > "$proof_dir/inside.sh" <<'SH2'
set -euo pipefail
[[ $XDG_RUNTIME_DIR == /work/run && ! -e /dev/snd ]]
export DBUS_SYSTEM_BUS_ADDRESS="$DBUS_SESSION_BUS_ADDRESS"
gsettings set org.gnome.shell enabled-extensions "['cadiswave-status@cadis.digital', 'cadiswave-test@cadis.digital']"
gnome-shell --headless --wayland --virtual-monitor 1024x768 --no-x11 > /work/shell.log 2>&1 &
shell_pid=$!
trap 'kill "$shell_pid" 2>/dev/null || true' EXIT
python3 /work/check.py
SH2
status=0
bwrap --unshare-user --unshare-pid --unshare-net --unshare-ipc --unshare-uts --die-with-parent --new-session --cap-drop ALL --clearenv \
 --ro-bind /usr /usr --ro-bind /bin /bin --ro-bind /lib /lib --ro-bind /lib64 /lib64 --ro-bind /etc /etc --dev /dev --proc /proc --tmpfs /tmp \
 --bind "$proof_dir" /work --setenv PATH /usr/bin:/bin --setenv HOME /work/home --setenv XDG_RUNTIME_DIR /work/run \
 --setenv XDG_DATA_DIRS /usr/share --setenv XDG_SESSION_TYPE wayland --setenv GDK_BACKEND wayland --setenv WAYLAND_DISPLAY wayland-0 --setenv LIBGL_ALWAYS_SOFTWARE 1 \
 -- dbus-run-session bash /work/inside.sh > "$proof_dir/result.log" 2>&1 || status=$?
cat "$proof_dir/result.log"
printf 'GNOME evidence: %s\n' "$proof_dir"
exit "$status"
