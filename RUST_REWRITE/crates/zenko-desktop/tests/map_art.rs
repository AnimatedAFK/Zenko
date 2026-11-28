//! mapart scenes against zenko.py (golden/map.json from tools/make_golden_map.py): every pixel row (FNV-1a 64),
//! the projection scale, the Okinawa inset and six journey pins, for Japan, Kyoto (4 seasons), myth, Korea, China, Silk Road.

use serde_json::Value;
use zenko_desktop::mapart::region_scene;

fn fnv(b: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for &x in b {
        h = (h ^ x as u64).wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

#[test]
fn map_scenes_match_python() {
    let g: Vec<Value> = serde_json::from_str(include_str!("../../../golden/map.json")).unwrap();
    let all = zenko_desktop::mapart::mdata()["JOURNEY_ALL"].as_array().unwrap();
    let mut bad = Vec::new();
    for c in &g {
        let a = c["args"].as_array().unwrap();
        let (region, w, h) = (a[0].as_str().unwrap(), a[1].as_i64().unwrap(), a[2].as_i64().unwrap());
        let (r, p) = region_scene(region, w, h, a[3].as_str().unwrap_or(""));
        let row = (w * 3) as usize;
        let rows = c["rows"].as_array().unwrap();
        let diff: Vec<usize> = (0..h as usize).filter(|&y| fnv(&r.b[y * row..(y + 1) * row]) != rows[y].as_str().unwrap()).collect();
        if !diff.is_empty() {
            bad.push(format!("{:?}: {} rows differ, first {:?}", a, diff.len(), &diff[..diff.len().min(6)]));
        }
        assert_eq!(p.s(), c["s"].as_f64().unwrap(), "s {:?}", a);
        let i = p.inset();
        let gi: Vec<i64> = c["inset"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect();
        assert_eq!(vec![i.0, i.1, i.2, i.3], gi, "inset {:?}", a);
        for (k, pt) in c["pts"].as_array().unwrap().iter().enumerate() {
            let e = all[k].as_array().unwrap();
            let (x, y) = p.xy(e[4].as_f64().unwrap(), e[5].as_f64().unwrap());
            assert_eq!((x, y), (pt[0].as_f64().unwrap(), pt[1].as_f64().unwrap()), "pt {k} {:?}", a);
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
