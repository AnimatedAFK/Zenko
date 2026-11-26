"""crates/zenko-desktop/data/life.json: the lines and lists zenko.py's desktop life uses (haiku by season, found
things, night/hungry/pouty/happy lines, visiting foxes, weather colours, the music tablet's icons and colours, music
tracks), taken from the untouched zenko.py."""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402

data = {
    "HAIKU": Z.HAIKU, "FOUND_THINGS": Z.FOUND_THINGS, "NIGHT_LINES": Z.NIGHT_LINES, "HUNGRY_LINES": Z.HUNGRY_LINES,
    "POUTY_LINES": Z.POUTY_LINES, "HAPPY_LINES": Z.HAPPY_LINES, "VISITORS": [list(v) for v in Z.VISITORS],
    "WX_LOOK": {k: list(v) for k, v in Z.FoxBuddy.WX_LOOK.items()},
    "TAB_PAL": Z.TAB_PAL, "TAB": [Z.TAB_X, Z.TAB_Y, Z.TAB_W, Z.TAB_H, Z.TAB_B],
    "TAB_BUTTONS": {k: list(v) for k, v in Z.TAB_BUTTONS.items()},
    "MUSIC_TRACKS": [list(t) for t in Z.MUSIC_TRACKS], "COAT_KEYS": Z.COAT_KEYS,
}
out = os.path.join(os.path.dirname(__file__), "..", "crates", "zenko-desktop", "data", "life.json")
with open(out, "w", encoding="utf-8") as f:
    json.dump(data, f, ensure_ascii=False, indent=0)
print({k: len(v) for k, v in data.items()})
