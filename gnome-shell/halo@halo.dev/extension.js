// Halo: Alt+clic su un file in Nautilus o sul desktop apre il menu radiale.
//
// Su Wayland i clic sulle finestre delle app non passano sempre da GNOME Shell,
// quindi l'estensione legge lo stato di mouse e tasti a intervalli brevi e,
// quando vede premere il tasto sinistro con Alt, chiede a halo-pick quale file
// c'è sotto il puntatore. Il clic arriva comunque anche all'app.
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const POLL_MS = 35;
const PAUSE_US = 800 * 1000;
const M = Clutter.ModifierType;
const OTHER_MODS = M.SHIFT_MASK | M.CONTROL_MASK | M.SUPER_MASK | M.MOD4_MASK;

export default class HaloExtension extends Extension {
    enable() {
        this._down = false;
        this._last = 0;
        this._timer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, POLL_MS, () => {
            this._tick();
            return GLib.SOURCE_CONTINUE;
        });
    }

    disable() {
        if (this._timer)
            GLib.source_remove(this._timer);
        this._timer = 0;
    }

    _tick() {
        const [x, y, mods] = global.get_pointer();
        const down = (mods & M.BUTTON1_MASK) !== 0;
        const pressed = down && !this._down;
        this._down = down;
        if (!pressed || !(mods & M.MOD1_MASK) || (mods & OTHER_MODS))
            return;

        const now = GLib.get_monotonic_time();
        if (now - this._last < PAUSE_US)
            return;
        const win = windowAt(x, y);
        const app = win && appOf(win);
        if (!app)
            return;
        this._last = now;

        const r = win.get_frame_rect();
        // Il menu si apre in una finestra massimizzata sul monitor del puntatore: la sua
        // posizione è relativa all'area di lavoro di quel monitor (senza la barra in alto).
        const mon = global.display.get_current_monitor();
        const wa = global.workspace_manager.get_active_workspace().get_work_area_for_monitor(mon);
        const argv = [GLib.build_filenamev([this.path, 'halo-pick']),
            '--app', app, '--title', win.get_title() ?? '',
            '--window-path', win.get_gtk_window_object_path() ?? '',
            '--x', String(x - r.x), '--y', String(y - r.y),
            '--menu-x', String(x - wa.x), '--menu-y', String(y - wa.y)];
        console.log(`Halo: Alt+clic su ${app} in ${x},${y}`);
        try {
            Gio.Subprocess.new(argv, Gio.SubprocessFlags.NONE);
        } catch (e) {
            console.error(`Halo: impossibile avviare halo-pick: ${e.message}`);
        }
    }
}

/** La finestra più in alto sotto il punto, nell'area di lavoro attiva. */
function windowAt(x, y) {
    const ws = global.workspace_manager.get_active_workspace();
    const wins = global.display.sort_windows_by_stacking(ws.list_windows()).reverse();
    return wins.find(w => {
        if (w.minimized)
            return false;
        const r = w.get_frame_rect();
        return x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height;
    }) ?? null;
}

/** "nautilus", "desktop" (Desktop Icons NG) o null per le altre finestre. */
function appOf(win) {
    const cls = (win.get_wm_class() ?? '').toLowerCase();
    if (cls === 'org.gnome.nautilus')
        return 'nautilus';
    if ((win.get_title() ?? '').startsWith('Desktop Icons') || cls.includes('ding'))
        return 'desktop';
    return null;
}
