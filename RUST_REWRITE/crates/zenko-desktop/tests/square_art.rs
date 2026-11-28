//! squareart against zenko.py (golden/square.json from tools/make_golden_square.py): the festival street for every
//! festival in day, dusk and night (every pixel row and the layout), the crowd's looks and sprite rows, the festival
//! calendar, and the Town square's filters.

use serde_json::Value;
use zenko_desktop::squareart::*;
use zenko_sound::mt::PyRandom;

fn fnv(b: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for &x in b {
        h = (h ^ x as u64).wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn g() -> Value {
    serde_json::from_str(include_str!("../../../golden/square.json")).unwrap()
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

#[test]
fn streets_match_python() {
    let g = g();
    let mut bad = vec![];
    for c in g["streets"].as_array().unwrap() {
        let a = c["args"].as_array().unwrap();
        let (aw, ah) = (a[0].as_i64().unwrap(), a[1].as_i64().unwrap());
        let (r, lay) = street(aw, ah, a[2].as_str().unwrap(), a[3].as_str().unwrap(), a[4].as_i64().unwrap());
        let row = (aw * 3) as usize;
        let rows = c["rows"].as_array().unwrap();
        let diff: Vec<usize> = (0..ah as usize).filter(|&y| fnv(&r.b[y * row..(y + 1) * row]) != rows[y].as_str().unwrap()).collect();
        if !diff.is_empty() {
            bad.push(format!("{a:?}: {} rows differ, first {:?}", diff.len(), &diff[..diff.len().min(5)]));
            continue;
        }
        let l = &c["lay"];
        assert_eq!(lay.gy, l["gy"].as_i64().unwrap());
        assert_eq!(lay.mid, l["mid"].as_i64().unwrap());
        let sky: Vec<i64> = l["sky"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect();
        assert_eq!(lay.sky, sky, "{a:?} sky");
        let ln = l["lanterns"].as_array().unwrap();
        assert_eq!(lay.lanterns.len(), ln.len(), "{a:?} lanterns");
        for (x, w) in lay.lanterns.iter().zip(ln) {
            assert_eq!((x.0, x.1), (f(&w[0]), f(&w[1])), "{a:?} lantern");
            assert_eq!(vec![x.2.0 as i64, x.2.1 as i64, x.2.2 as i64],
                       w[2].as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect::<Vec<_>>());
        }
        let tw = l["twinkles"].as_array().unwrap();
        assert_eq!(lay.twinkles.len(), tw.len(), "{a:?} twinkles");
        for (x, w) in lay.twinkles.iter().zip(tw) {
            assert_eq!((x.0, x.1, x.2.as_str()), (f(&w[0]), f(&w[1]), w[2].as_str().unwrap()), "{a:?} twinkle");
        }
        let st = l["stalls"].as_array().unwrap();
        assert_eq!(lay.stalls.len(), st.len());
        for (s, w) in lay.stalls.iter().zip(st) {
            assert_eq!((s.key.as_str(), s.x0, s.x1, s.sign_y, s.top), (w["key"].as_str().unwrap(), w["x0"].as_i64().unwrap(),
                       w["x1"].as_i64().unwrap(), w["sign_y"].as_i64().unwrap(), w["top"].as_i64().unwrap()));
        }
        let tr: Vec<i64> = l["trees"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect();
        assert_eq!(lay.trees, tr);
        assert_eq!(lay.lanes, (l["lanes"][0].as_i64().unwrap(), l["lanes"][1].as_i64().unwrap()));
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn people_match_python() {
    let g = g();
    for p in g["people"].as_array().unwrap() {
        let l = look(&mut PyRandom::new(p["seed"].as_u64().unwrap()), p["ev"].as_str().unwrap(), p["fox"].as_bool().unwrap());
        let w = &p["look"];
        let j = l.to_json();
        for k in ["h", "s", "y", "o", "long", "bun", "kid", "acc"] {
            assert_eq!(j[k], w[k], "seed {} {k}", p["seed"]);
        }
        for k in ["hat", "ghost", "fox"] {
            let want = w.get(k).cloned().unwrap_or(if k == "hat" { Value::Null } else { Value::Bool(false) });
            assert_eq!(j[k], want, "seed {} {k}", p["seed"]);
        }
        for fr in 0..2 {
            let rows: Vec<String> = p["rows"][fr].as_array().unwrap().iter().map(|v| v.as_str().unwrap().into()).collect();
            assert_eq!(person_rows(&l, fr as i32), rows, "seed {} frame {fr}", p["seed"]);
        }
    }
    for u in g["uidlook"].as_array().unwrap() {
        let l = look(&mut PyRandom::new_str(u["uid"].as_str().unwrap()), "winter", true);
        assert_eq!(l.to_json()["y"], u["look"]["y"]);
        assert_eq!(l.to_json()["s"], u["look"]["s"]);
        assert_eq!(l.to_json()["acc"], u["look"]["acc"]);
    }
}

#[test]
fn calendar_and_filters_match_python() {
    let g = g();
    for d in g["days"].as_array().unwrap() {
        let day = zenko_desktop::plan::parse_iso(d[0].as_str().unwrap()).unwrap();
        assert_eq!(fest_today(day), d[1].as_str().unwrap(), "{d}");
        let nx = fest_next(day).map(|(k, a)| serde_json::json!([k, zenko_desktop::plan::iso(a)])).unwrap_or(Value::Null);
        assert_eq!(nx, d[2], "{d}");
    }
    for c in g["clean"].as_array().unwrap() {
        assert_eq!(square_clean(c[0].as_str().unwrap()), c[1].as_str().unwrap(), "{c}");
    }
    for c in g["names"].as_array().unwrap() {
        assert_eq!(square_name_ok(c[0].as_str().unwrap()).map(Value::from).unwrap_or(Value::Null), c[1], "{c}");
    }
    for c in g["tags"].as_array().unwrap() {
        assert_eq!(square_tag(c[0].as_str().unwrap()), c[1].as_str().unwrap());
    }
}
