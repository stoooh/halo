import shutil
import subprocess

from gi.repository import GObject, Nautilus


class HaloMenuProvider(GObject.GObject, Nautilus.MenuProvider):
    def get_file_items(self, files):
        # Uno o più file (selezione multipla): Halo li riceve tutti.
        if not files or any(f.get_uri_scheme() != "file" for f in files):
            return []
        item = Nautilus.MenuItem(
            name="Halo::azioni",
            label="Azioni rapide…",
            tip="Apri il menu radiale di Halo",
        )
        item.connect("activate", self._open, files)
        return [item]

    def _open(self, _menu, files):
        paths = [p for p in (f.get_location().get_path() for f in files) if p]
        exe = shutil.which("halo")
        if paths and exe:
            subprocess.Popen([exe, "--", *paths], start_new_session=True)
