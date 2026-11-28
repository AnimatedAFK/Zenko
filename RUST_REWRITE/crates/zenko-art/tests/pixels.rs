//! Palettes (make_pal) and the final RGBA images (incl. zenko21 HD smoothing) must match zenko.py exactly.
use flate2::read::GzDecoder;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader};
use zenko_art::palette::{LookCfg, make_pal};
use zenko_art::{Grid, Look, core, curl, raster, rig, sit};

fn cases() -> Vec<Value> {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/pixels.jsonl.gz");
    let f = std::fs::File::open(p).unwrap();
    BufReader::new(GzDecoder::new(f))
        .lines()
        .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
        .collect()
}

fn cfg(c: &Value) -> LookCfg {
    let g = |k: &str| {
        c["cfg"]
            .get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    LookCfg {
        theme: g("theme"),
        coat: g("coat"),
        eye_color: g("eye_color"),
        foxfire_color: g("foxfire_color"),
        scarf: g("scarf"),
        robe: c["robe"].as_str().unwrap().to_string(),
    }
}

fn sprite(name: &str) -> Vec<Grid> {
    let lk = Look::default();
    match name {
        "tails+frame" => vec![
            core::build_tails(3, 2, 8, false),
            sit::build_frame("happy", 1, None, &["glasses"], 0, 0, &lk),
        ],
        "rig" => vec![rig::rig_rows("rigR:playbow:1", 5, &["kimono_spring"], &lk)],
        "walk" => vec![rig::rig_rows("qwalkL3", 3, &[], &lk)],
        "curl" => vec![curl::build_curl(1, &[], 4)],
        "fire" => vec![curl::foxfire_rows(1)],
        _ => panic!(),
    }
}

#[test]
fn palettes_and_hd_pixels_match_python() {
    let mut bad = vec![];
    let (mut npal, mut nimg) = (0, 0);
    for c in cases() {
        let pal = make_pal(&cfg(&c));
        if c["kind"] == "pal" {
            npal += 1;
            let want = c["pal"].as_object().unwrap();
            for k in 0u8..128 {
                let ks = (k as char).to_string();
                let w = want
                    .get(&ks)
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_lowercase());
                if ks == "." {
                    continue;
                }
                if pal.hex(k) != w {
                    bad.push(format!(
                        "pal {:?} key {ks}: rust {:?} python {:?}",
                        c["cfg"],
                        pal.hex(k),
                        w
                    ));
                }
            }
        } else {
            nimg += 1;
            let layers = sprite(c["sprite"].as_str().unwrap());
            let refs: Vec<&Grid> = layers.iter().collect();
            let zoom = c["zoom"].as_u64().unwrap() as usize;
            let img = raster::sprite_photo(&refs, &pal, zoom, true);
            let mut h = Sha256::new();
            for p in &img.px {
                h.update(p);
            }
            let sha: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
            if (img.w, img.h)
                != (
                    c["w"].as_u64().unwrap() as usize,
                    c["h"].as_u64().unwrap() as usize,
                )
                || sha != c["sha"].as_str().unwrap()
            {
                bad.push(format!(
                    "img {} zoom {} {:?}: size {}x{}",
                    c["sprite"], zoom, c["cfg"], img.w, img.h
                ));
            }
        }
    }
    println!("{npal} palettes, {nimg} images checked");
    for b in bad.iter().take(15) {
        println!("  {b}");
    }
    assert!(bad.is_empty());
}
