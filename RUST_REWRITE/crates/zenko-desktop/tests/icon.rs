//! The window icon against zenko.py icon_pixels(64, theme) (golden/icon.json from tools/make_golden_icon.py).
#[test]
fn icon_matches_python() {
    let g: serde_json::Value = serde_json::from_str(include_str!("../../../golden/icon.json")).unwrap();
    for (th, rows) in g.as_object().unwrap() {
        let px = zenko_desktop::icon::icon_pixels(th);
        let mine: Vec<String> = px
            .chunks(64)
            .map(|r| r.iter().map(|p| format!("{:02x}{:02x}{:02x}{:02x}", p[0], p[1], p[2], p[3])).collect())
            .collect();
        let want: Vec<&str> = rows.as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        let bad = mine.iter().zip(&want).filter(|(a, b)| a != *b).count();
        assert_eq!(bad, 0, "{th}: {bad} rows differ");
    }
}
