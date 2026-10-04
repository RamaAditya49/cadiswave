import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Shell from 'gi://Shell';
import St from 'gi://St';
import {Extension, InjectionManager} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const APPLICATION = 'io.github.RamaAditya49.CadisWave';
const ACTION_PATH = '/io/github/RamaAditya49/CadisWave';
const COLORED_ICONS = new Set(['cadiswave-green', 'cadiswave-red', 'cadiswave-orange']);
const APP_IDS = new Set(['cadiswave.desktop', `${APPLICATION}.desktop`]);

export default class CadisWaveStatus extends Extension {
    enable() {
        this._icons = new Map();
        this._name = 'cadiswave';
        this._group = null;
        this._signals = [];
        this._injections = new InjectionManager();
        const extension = this;
        this._injections.overrideMethod(Shell.App.prototype, 'create_icon_texture', original => {
            return function (size) {
                const icon = original.call(this, size);
                if (APP_IDS.has(this.get_id()) && icon instanceof St.Icon)
                    extension._track(icon);
                return icon;
            };
        });
        // Include icons that the dock created before this extension was enabled.
        const queue = [Main.layoutManager.uiGroup];
        while (queue.length) {
            const actor = queue.pop();
            const delegate = actor._delegate ?? actor;
            if (APP_IDS.has(delegate.app?.get_id())) {
                const icon = delegate.icon?.icon;
                if (icon instanceof St.Icon) this._track(icon);
            }
            queue.push(...actor.get_children());
        }
        this._watcher = Gio.bus_watch_name(Gio.BusType.SESSION, APPLICATION, Gio.BusNameWatcherFlags.NONE,
            (connection, _, owner) => {
                this._disconnectGroup();
                this._group = Gio.DBusActionGroup.get(connection, owner, ACTION_PATH);
                for (const signal of ['action-added', 'action-removed', 'action-state-changed']) {
                    this._signals.push(this._group.connect(signal, (_, name) => {
                        if (name === 'status-icon') this._refresh();
                    }));
                }
                this._refresh();
            },
            () => {
                this._disconnectGroup();
                this._refresh();
            });
    }

    _disconnectGroup() {
        for (const signal of this._signals) this._group.disconnect(signal);
        this._signals = [];
        this._group = null;
    }

    _track(icon) {
        if (this._icons.has(icon)) return;
        this._icons.set(icon, {
            original: icon.gicon,
            destroy: icon.connect('destroy', () => this._icons.delete(icon)),
        });
        this._apply(icon);
    }

    _apply(icon) {
        if (this._name === 'cadiswave') {
            icon.gicon = this._icons.get(icon).original;
        } else {
            icon.gicon = new Gio.FileIcon({file: this.dir.get_child(`${this._name}.svg`)});
        }
    }

    _refresh() {
        const state = this._group?.has_action('status-icon') ? this._group.get_action_state('status-icon') : null;
        const name = state?.is_of_type(new GLib.VariantType('s')) ? state.get_string()[0] : '';
        const next = COLORED_ICONS.has(name) ? name : 'cadiswave';
        if (next === this._name) return;
        this._name = next;
        for (const icon of this._icons.keys()) this._apply(icon);
    }

    disable() {
        if (this._watcher) Gio.bus_unwatch_name(this._watcher);
        this._watcher = null;
        this._disconnectGroup();
        this._injections?.clear();
        this._injections = null;
        for (const [icon, state] of this._icons ?? []) {
            icon.gicon = state.original;
            icon.disconnect(state.destroy);
        }
        this._icons = null;
        this._name = null;
    }
}
