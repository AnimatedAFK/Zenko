//! round_pts must give the same outline as zenko.py (after Tk's own rounding of canvas coordinates).

fn tk_coord(v: f64) -> i32 {
    if v > 0.0 { (v + 0.5) as i32 } else { (v - 0.5) as i32 }
}

#[test]
fn round_pts_matches_python() {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/ui_shapes.json");
    let cases: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    let mut bad = 0;
    for c in &cases {
        let g = |k: &str| c[k].as_i64().unwrap() as i32;
        let got = zenko_desktop::shapes::round_pts(g("x1"), g("y1"), g("x2"), g("y2"), g("r"), g("step"),
                                                   c["shape"].as_str().unwrap());
        let flat: Vec<f64> = c["pts"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
        let want: Vec<(i32, i32)> = flat.chunks(2).map(|p| (tk_coord(p[0]), tk_coord(p[1]))).collect();
        if got != want {
            bad += 1;
            if bad < 4 {
                println!("{c}\n got  {got:?}\n want {want:?}");
            }
        }
    }
    println!("{} shapes checked, {} different", cases.len(), bad);
    assert_eq!(bad, 0);
}
