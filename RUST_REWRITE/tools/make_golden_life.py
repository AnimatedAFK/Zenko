"""golden/life.json: zenko.py's own answers for the music tablet picture (draw_tablet, several songs / states /
flashes / scroll times), tablet_hit, parse_clock, the weather URLs and geocode's pick, made by calling zenko.py's
methods on a bare object. Checked by crates/zenko-desktop/tests/life_bits.rs."""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile, types
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402

MONO = [0.0]
Z.time.monotonic = lambda: MONO[0]
got = {}
Z.sprite_photo = lambda layers, pal, zoom=1, bg=None: got.setdefault("rows", layers[0])


class Canvas:
    def __getattr__(self, k):
        return lambda *a, **kw: None


o = types.SimpleNamespace()
for k in ("tablet_grid", "draw_tablet", "tablet_hit"):
    setattr(o, k, types.MethodType(getattr(Z.FoxBuddy, k), o))
o.canvas, o.pal, o.Z, o.COLW, o.fox_y, o.cfg = Canvas(), {}, 4, 37, 21, {}
o.is_curled = lambda: False
tab = []
for player, flash, t in [({"playing": True, "title": "Lemon", "artist": "Kenshi Yonezu"}, ("", 0.0), 12.34),
                         ({"playing": False, "title": "Spotify", "artist": ""}, ("playpause", 99.9), 100.0),
                         ({"playing": True, "title": "Café Ñandú – 夜に駆ける", "artist": "YOASOBI"}, ("next", 5.0), 5.1),
                         ({"playing": True, "title": "Hi", "artist": ""}, ("previous", 0.0), 7.77),
                         ({"playing": False, "title": "A very long song title that has to scroll along", "artist": "Band"},
                          ("", 0.0), 3.0)]:
    MONO[0] = t
    o.player, o.tab_flash, o._tab_key = player, flash, None
    got.clear()
    o.draw_tablet(0, 0, True)
    tab.append({"player": player, "flash": list(flash), "t": t, "rows": got["rows"]})
hits = []
o.player = {"playing": True, "title": "x", "artist": ""}
for x in range(30, 37 + 4 * 60, 7):
    for y in range(21 + 4 * 45, 21 + 4 * 76, 5):
        hits.append([x, y, o.tablet_hit(x, y)])
clocks = {s: Z.parse_clock(s) for s in ["4pm", "16:30", "4.30 pm", "9", "12am", "12pm", "13pm", "0am", "24:00", "7:5",
                                        " 7:05 ", "11a", "x", "", "5 p", "23:59", "09:60"]}
geo_res = {"results": [{"name": "Paris", "country": "France", "country_code": "FR", "admin1": "Île-de-France",
                        "latitude": 48.85341, "longitude": 2.3488},
                       {"name": "Paris", "country": "United States", "country_code": "US", "admin1": "Texas",
                        "latitude": 33.66094, "longitude": -95.55551}]}
geo = {}
for place in ["Paris", "Paris, Texas", "Paris, us", "Paris, Germany"]:
    Z.http_json = lambda url, timeout=12: (geo.setdefault("urls", []).append(url), geo_res)[1]
    geo[place] = Z.geocode(place)
urls = []
Z.http_json = lambda url, timeout=12: (urls.append(url), {"current": {"weather_code": 61}})[1]
w = types.SimpleNamespace(cfg={"home_city": "Paris, Texas"}, wx={"geo": None})
w.bg = lambda job, done: done(job(), None)
w.root = types.SimpleNamespace(after=lambda *a: None)
w.weather_poll = None
Z.geocode = lambda c: dict(geo_res["results"][1])
Z.FoxBuddy.weather_poll(w)
out = {"tablet": tab, "hits": hits, "clocks": clocks, "geo": geo, "forecast_url": urls[0], "code": w.wx["code"],
       "quote": [Z._q(s) for s in ["São Paulo", "New York", "a/b&c=d", "東京", "x-y_z.~"]]}
with open(os.path.join(os.path.dirname(__file__), "..", "golden", "life.json"), "w", encoding="utf-8") as f:
    json.dump(out, f, ensure_ascii=False, indent=0)
print(len(tab), len(hits), clocks)
