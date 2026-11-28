//! gardenart::garden_scene against zenko.py (golden/garden.json from tools/make_golden_garden.py): every pixel row
//! (FNV-1a 64 hash) and the layout, for a spread of sizes, stages, health, grove, seeds, seasons, sky and weather.

use serde_json::Value;
use zenko_desktop::gardenart::{Look, garden_scene};

fn fnv(b: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for &x in b {
        h = (h ^ x as u64).wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn close(a: f64, b: &Value, what: &str) {
    let b = b.as_f64().unwrap();
    assert!(a == b, "{what}: {a} vs {b}");
}

#[test]
fn garden_scene_matches_python() {
    let g: Vec<Value> = serde_json::from_str(include_str!("../../../golden/garden.json")).unwrap();
    let mut bad = Vec::new();
    for c in &g {
        let a = c["args"].as_array().unwrap();
        let (w, h) = (a[0].as_i64().unwrap(), a[1].as_i64().unwrap());
        let look = Look {
            season: a[10].as_bool().unwrap(), sky: a[11].as_bool().unwrap(), season_name: a[7].as_str().unwrap().into(),
            part: a[8].as_str().unwrap().into(), weather: a[9].as_str().map(String::from),
        };
        let (r, lay) = garden_scene(w, h, a[2].as_i64().unwrap(), a[3].as_f64().unwrap(), a[4].as_f64().unwrap(),
            a[5].as_i64().unwrap(), a[6].as_u64().unwrap(), &look);
        let rows = c["rows"].as_array().unwrap();
        let row = (w * 3) as usize;
        let diff: Vec<usize> = (0..h as usize).filter(|&y| fnv(&r.b[y * row..(y + 1) * row]) != rows[y].as_str().unwrap()).collect();
        if !diff.is_empty() {
            bad.push(format!("{:?}: {} rows differ, first {:?}", a, diff.len(), &diff[..diff.len().min(6)]));
        }
        let tips = c["tips"].as_array().unwrap();
        assert_eq!(tips.len(), lay.tips.len(), "tips count {:?}", a);
        for (t, e) in lay.tips.iter().zip(tips) {
            close(t.0, &e[0], "tip x");
            close(t.1, &e[1], "tip y");
            assert_eq!(t.2, e[2].as_i64().unwrap());
        }
        close(lay.lanterns[0].1, &c["lanterns"][0][1], "lantern");
        close(lay.pond.0, &c["pond"][0], "pond");
        close(lay.moon.2, &c["moon"][2], "moon");
        close(lay.basin.2, &c["basin"][2], "basin");
        close(lay.breath.1, &c["breath"][1], "breath");
        assert_eq!(lay.wall, c["wall"].as_i64().unwrap());
        assert_eq!(lay.horizon, c["horizon"].as_i64().unwrap());
        assert_eq!(lay.tree.1, c["tree"][1].as_i64().unwrap());
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
