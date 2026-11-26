//! The Tk grid/pack maths in tkgrid.rs against where real Tk put every widget of zenko.py's new-session dialog
//! (golden/dialog_layout.json from tools/make_golden_dialog.py, run under xvfb-run with Python's tkinter).
use serde_json::Value;
use zenko_desktop::tkgrid::{Cell, grid, pack_left, pack_top_w};

fn geo(v: &Value) -> (i32, i32, i32, i32) {
    let g = |k: &str| v[k].as_i64().unwrap() as i32;
    (g("x"), g("y"), g("w"), g("h"))
}
fn req(v: &Value) -> (i32, i32) {
    (v["rw"].as_i64().unwrap() as i32, v["rh"].as_i64().unwrap() as i32)
}

#[test]
fn dialog_layout_matches_tk() {
    let g: Value = serde_json::from_str(include_str!("../../../golden/dialog_layout.json")).unwrap();
    for top in g.as_array().unwrap() {
        let frm = &top["kids"][0];
        let k = frm["kids"].as_array().unwrap();
        let (hdr, lab, ent, labm, entm, pre, start) = (&k[0], &k[1], &k[2], &k[3], &k[4], &k[5], &k[6]);
        // inside the header
        let tt = &hdr["kids"][1];
        let lines: Vec<(i32, i32)> = tt["kids"].as_array().unwrap().iter().map(req).collect();
        let (tts, ttp) = pack_top_w(&lines);
        assert_eq!(tts, req(tt));
        for (p, v) in ttp.iter().zip(tt["kids"].as_array().unwrap()) {
            assert_eq!((p.0, p.1), (geo(v).0, geo(v).1));
        }
        let icon = req(&hdr["kids"][0]);
        let (hs, hp) = pack_left(&[(icon.0, icon.1, 0, 10), (tts.0, tts.1, 0, 0)]);
        assert_eq!(hs, req(hdr));
        assert_eq!(hp[0], (geo(&hdr["kids"][0]).0, geo(&hdr["kids"][0]).1));
        assert_eq!(hp[1], (geo(tt).0, geo(tt).1));
        let items: Vec<(i32, i32, i32, i32)> =
            pre["kids"].as_array().unwrap().iter().map(|b| { let r = req(b); (r.0, r.1, 2, 2) }).collect();
        let (ps, pp) = pack_left(&items);
        assert_eq!(ps, req(pre));
        for (p, v) in pp.iter().zip(pre["kids"].as_array().unwrap()) {
            assert_eq!(*p, (geo(v).0, geo(v).1));
        }
        let c = |v: &Value, r, col| { let q = req(v); Cell::new(r, col, q.0, q.1) };
        let cells = vec![
            c(hdr, 0, 0).span(4).sticky("w").pady(0, 10),
            c(lab, 1, 0).span(4).sticky("w"),
            c(ent, 2, 0).span(4).pady(4, 12).sticky("ew").ipad(0, 3),
            c(labm, 3, 0).sticky("w"),
            c(entm, 3, 1).sticky("w").ipad(0, 2),
            c(pre, 3, 2).span(2).sticky("e"),
            c(start, 4, 0).span(4).pady(16, 0).sticky("ew"),
        ];
        let ((w, h), at) = grid(&cells, 20, 16);
        assert_eq!((w + 40, h + 32), req(frm));
        for (i, (a, v)) in at.iter().zip(k).enumerate() {
            assert_eq!(*a, geo(v), "widget {i}");
        }
    }
}
