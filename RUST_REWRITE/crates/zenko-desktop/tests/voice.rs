//! The voice layer (voice.rs) against zenko.py (golden/voice.json from tools/make_golden_voice.py): jp_clean,
//! jp_tics_only, jp_fallback, vv_say_text, the microphone's voice detector, mic_peak, pcm_to_wav, whisper's output
//! cleaning, stt_paths and voicevox_find_engine on made-up folders, and the zip reader stt_install uses.

use serde_json::Value;
use std::path::{Path, PathBuf};
use zenko_desktop::voice::*;

fn golden() -> Value {
    serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/voice.json")).unwrap()).unwrap()
}

const TEXTS: &[&str] = &["Kon kon~ ehehe!", "Kon!", "Yatta~! You did it!", "*wags tail* Let's focus on Biology now. You have 25 minutes left!",
    "Here's the link: https://example.com/page and the formula $E=mc^2$ ok?", "**Bold** and _italic_ with `code` # head",
    "Mou~ hmph!", "Fufu, sugoi ne~", "Ganbatte! 🦊✨ You can do it.", "What do you want to study?",
    "I'm so tired and sad today", "Well done! Proud of you.", "Back to work, close YouTube.", "Ehehe kawaii",
    "Mata ne~ oyasumi!", "こんにちは！元気？", "", "", "Line one\nLine two\n- bullet item\n1. numbered", "> quote | table | cell",
    "Hmm... okay?!", "", "   ", "Nee~ what's next?", "Itadakimasu! nom nom", "Gyuu~ *hugs you tight*", "uuu... auu", "kyuun kyuun",
    "Arigatou! Konnichiwa! Ohayou!", "Take a 5 minute break, then 25 more. [Start](cmd:focus)",
    "The answer is \\(x^2\\) and {curly} [square] stuff~ ***triple***", "Hello? Are you there? Hello?"];

fn texts() -> Vec<String> {
    let mut v: Vec<String> = TEXTS.iter().map(|s| s.to_string()).collect();
    v[16] = "A ".repeat(120) + "end.";
    v[17] = "First sentence. ".repeat(20);
    v
}

#[test]
fn jp_text_matches_zenko_py() {
    let g = golden();
    let ts = texts();
    let mut bad = vec![];
    let mut ti = 0;
    for (i, t) in ts.iter().enumerate() {
        let c = jp_clean(t);
        if c != g["clean"][i].as_str().unwrap() { bad.push(format!("clean {i}: {c:?} vs {}", g["clean"][i])); }
        if !c.is_empty() {
            let x = jp_tics_only(&c);
            if x != g["tics"][ti].as_str().unwrap() { bad.push(format!("tics {i}: {x:?} vs {}", g["tics"][ti])); }
            ti += 1;
        }
        let s = vv_say_text(t);
        let want: Vec<String> = g["say"][i].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
        if s != want { bad.push(format!("say {i}: {s:?} vs {want:?}")); }
        let fb = &g["fallback"][i];
        let mut seen: Option<Vec<String>> = None;
        let r = jp_fallback(t, &mut |seq| { if seen.is_none() { seen = Some(seq.iter().map(|s| s.to_string()).collect()); } seq[0] });
        if r != fb["r"].as_str().unwrap() { bad.push(format!("fallback {i}: {r:?} vs {}", fb["r"])); }
        let wc: Option<Vec<String>> = fb["choices"].as_array().map(|a| a.iter().map(|v| v.as_str().unwrap().to_string()).collect());
        if seen != wc { bad.push(format!("choices {i}: {seen:?} vs {wc:?}")); }
    }
    assert_eq!(ti, g["tics"].as_array().unwrap().len());
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

fn frames(levels: &[i64]) -> Vec<Vec<u8>> {
    levels.iter().enumerate().map(|(k, &lv)| {
        (0..1600).flat_map(|i| (((lv as f64) * ((i as f64) * 0.3 + k as f64).sin()) as i64 as i16).to_le_bytes()).collect()
    }).collect()
}

#[test]
fn ears_match_zenko_py() {
    let g = golden();
    for (name, v) in g["vad"].as_object().unwrap() {
        let auto = name.ends_with("_1");
        let lv: Vec<i64> = v["levels"].as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect();
        let mut vad = Vad::default();
        let res: Vec<bool> = frames(&lv).iter().map(|f| vad.feed(f, auto, 40.0)).collect();
        let want: Vec<bool> = v["res"].as_array().unwrap().iter().map(|x| x.as_bool().unwrap()).collect();
        assert_eq!(res, want, "{name}");
        assert_eq!(vad.heard, v["heard"].as_bool().unwrap(), "{name}");
        let st = &v["st"];
        assert_eq!((vad.n, vad.loud, vad.quiet), (st["n"].as_i64().unwrap(), st["loud"].as_i64().unwrap(), st["quiet"].as_i64().unwrap()), "{name}");
        assert!((vad.floor - st["floor"].as_f64().unwrap()).abs() < 1e-9, "{name} floor {} vs {}", vad.floor, st["floor"]);
    }
    let mut pk: Vec<i32> = frames(&[0, 10, 3000, 32767]).iter().map(|f| mic_peak(f)).collect();
    pk.push(mic_peak(b""));
    pk.push(mic_peak(b"\x01"));
    let want: Vec<i32> = g["peak"].as_array().unwrap().iter().map(|x| x.as_i64().unwrap() as i32).collect();
    assert_eq!(pk, want);
    let p = std::env::temp_dir().join(format!("zk_wav_{}.wav", std::process::id()));
    pcm_to_wav(b"\x01\x02\x03\x04", &p, 16000).unwrap();
    let want: Vec<u8> = g["wav"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u8).collect();
    assert_eq!(std::fs::read(&p).unwrap(), want);
    for o in g["stt_out"].as_array().unwrap() {
        assert_eq!(stt_clean(o[0].as_str().unwrap()), o[1].as_str().unwrap());
    }
}

fn touch(root: &Path, rel: &str, size: usize) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, vec![b'x'; size]).unwrap();
}

fn rel(root: &Path, p: Option<PathBuf>) -> Value {
    match p {
        Some(p) => Value::String(p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/")),
        None => Value::Null,
    }
}

#[test]
fn finders_match_zenko_py() {
    let _g = zenko_desktop::web::test_lock(); // APPDATA is shared with the other tests
    let g = golden();
    let base = std::env::temp_dir().join(format!("zk_voice_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    unsafe { std::env::set_var("APPDATA", base.join("ad")) };
    let root = base.join("ad").join("FoxBuddy");
    for (r, sz) in [("speech/bin/Release/whisper-cli.exe", 5), ("speech/ggml-small.en-q5_1.bin", 50), ("speech/ggml-tiny.bin", 90),
                    ("speech/deep/a/b/c/d/main.exe", 5), ("extra/main.exe", 5), ("extra/whisper.dll", 5), ("extra/ggml-base.en-q5_1.bin", 20)] {
        touch(&root, r, sz);
    }
    for row in g["stt_paths"].as_array().unwrap() {
        let (k, d) = (row[0].as_str().unwrap(), row[1].as_str().unwrap());
        let extra = if d.is_empty() { String::new() } else { root.join(d).to_string_lossy().into_owned() };
        let (c, m) = stt_paths(k, &extra);
        assert_eq!(Value::Array(vec![rel(&root, c), rel(&root, m)]), row[2], "{k} {d}");
    }
    let eng = base.join("eng");
    for r in ["voicevox_engine/windows-cpu/run.exe", "voicevox_engine/run.exe", "Downloads/VOICEVOX-0.20/vv-engine/run.exe",
              "Downloads/voicevox_cpu/a/b/c/d/e/run.exe", "Desktop/MyVoicevox/x/run.exe"] {
        touch(&eng, r, 0);
    }
    let pl = Places { home: eng.clone(), local_app_data: eng.join("LocalAppData"), program_files: eng.join("Program Files"),
                      app_dir: eng.join("Apps").join("Zenko") };
    let r = |s: String| if s.is_empty() { String::new() } else { Path::new(&s).strip_prefix(&eng).unwrap().to_string_lossy().replace('\\', "/") };
    let r1 = r(find_engine("", &pl));
    let r2 = r(find_engine(&eng.join("Desktop/MyVoicevox/x/run.exe").to_string_lossy(), &pl));
    std::fs::remove_dir_all(eng.join("voicevox_engine")).unwrap();
    let r3 = r(find_engine("", &pl));
    assert_eq!(serde_json::json!([r1, r2, r3]), g["engine"]);
}

#[test]
fn zip_reader_reads_python_zips() {
    // made by Python's zipfile (one stored file, one deflated file, one folder)
    let out = std::process::Command::new("/usr/bin/python3.12").args(["-I", "-c",
        "import zipfile,io,sys;b=io.BytesIO();z=zipfile.ZipFile(b,'w');z.writestr('bin/Release/whisper-cli.exe',b'MZ'*10);\
z.writestr(zipfile.ZipInfo('bin/'),b'');z.writestr('a.txt','hello '*500,compress_type=zipfile.ZIP_DEFLATED);z.close();sys.stdout.buffer.write(b.getvalue())"]).output();
    let Ok(out) = out else { return };
    if !out.status.success() { return; }
    let e = zenko_desktop::unzip::entries(&out.stdout).unwrap();
    assert_eq!(e.len(), 2);
    assert_eq!(e[0], ("bin/Release/whisper-cli.exe".to_string(), b"MZ".repeat(10)));
    assert_eq!(e[1], ("a.txt".to_string(), b"hello ".repeat(500)));
    assert!(zenko_desktop::unzip::entries(b"not a zip at all, sorry!").is_err());
}

fn tiny_wav(n: usize) -> Vec<u8> {
    let pcm = vec![7u8; n * 2];
    let mut o = b"RIFF".to_vec();
    o.extend((36 + pcm.len() as u32).to_le_bytes());
    o.extend(b"WAVEfmt ");
    o.extend(16u32.to_le_bytes());
    o.extend([1, 0, 1, 0]);
    o.extend(24000u32.to_le_bytes());
    o.extend(48000u32.to_le_bytes());
    o.extend([2, 0, 16, 0]);
    o.extend(b"data");
    o.extend((pcm.len() as u32).to_le_bytes());
    o.extend(pcm);
    o
}

#[test]
fn voicevox_pipeline_with_a_pretend_engine() {
    use std::sync::{Arc, Mutex};
    let _g = zenko_desktop::web::test_lock();
    let base = std::env::temp_dir().join(format!("zk_vv_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    unsafe { std::env::set_var("APPDATA", &base) };
    let seen = Arc::new(Mutex::new(vec![]));
    let s2 = seen.clone();
    zenko_desktop::web::set_fetch(Arc::new(move |r, chunk| {
        s2.lock().unwrap().push(format!("{} {}", r.method, r.url));
        let body = if r.url.contains("/audio_query") { br#"{"speedScale":1.0}"#.to_vec() }
            else if r.url.contains("/synthesis") {
                let j: Value = serde_json::from_slice(&r.body).unwrap();
                assert_eq!((j["pitchScale"].as_f64(), j["speedScale"].as_f64(), j["intonationScale"].as_f64()), (Some(0.04), Some(1.05), Some(1.25)));
                tiny_wav(24000)
            } else if r.url.ends_with("/speakers") { br#"[{"name":"Zundamon","styles":[{"id":3,"name":"Normal"},{"id":1,"name":"Sweet"}]}]"#.to_vec() }
            else { return Err("<urlopen error [WinError 10061]>".into()) };
        chunk(&body, body.len() as u64);
        Ok(200)
    }));
    assert_eq!(speakers(2.0), Some(vec![(3, "Zundamon (Normal)".into()), (1, "Zundamon (Sweet)".into())]));
    assert!(!vv_alive(1.0));
    let parts = vec!["Hello there.".to_string(), "How are you?".to_string()];
    let gen_now = state().vv_gen;
    match vv_prepare(&parts, 3, gen_now) {
        Ready::Play(p, secs) => {
            assert!(p.ends_with(format!("speech_tmp/reply{}.wav", gen_now % 2)));
            assert!((secs - 2.0).abs() < 1e-9, "{secs}");
        }
        _ => panic!("no voice"),
    }
    assert!(base.join("FoxBuddy/speech_cache/91e30c17db322ab93990.wav").is_file(), "sha1 cache name as zenko.py");
    let n = seen.lock().unwrap().len();
    // the second time it comes from the cache: no engine calls at all
    assert!(matches!(vv_prepare(&parts[..1], 3, gen_now), Ready::Play(_, s) if (s - 1.0).abs() < 1e-9));
    assert_eq!(seen.lock().unwrap().len(), n);
    assert!(seen.lock().unwrap()[2].starts_with("POST http://127.0.0.1:50021/audio_query?text=Hello%20there.&speaker=3"), "{:?}", seen.lock().unwrap());
    assert!(matches!(vv_prepare(&parts, 3, gen_now + 1), Ready::Cancelled));
    assert!(matches!(vv_prepare(&["New line.".to_string()], 9, gen_now), Ready::Play(..)));
    zenko_desktop::web::set_fetch(Arc::new(|_, _| Err("<urlopen error [WinError 10061]>".into())));
    assert!(matches!(vv_prepare(&["Not cached.".to_string()], 3, gen_now), Ready::Down));
}
