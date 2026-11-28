"""crates/zenko-desktop/data/voice.json (zenko.py's voice tables and helper scripts: VOX_JP, VOICE_FOR, JP_TICS, the
Piper / Windows speech / Whisper workers, PIPER_VOICES, HOTKEYS, STT models) and golden/voice.json: jp_clean,
jp_tics_only, jp_fallback's fixed answers, vv_say_text, the microphone's voice detector, mic_peak, pcm_to_wav,
whisper's output cleaning, stt_paths and voicevox_find_engine on made-up folders."""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile, random, re, math, struct
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
os.environ.setdefault("APPDATA", tempfile.mkdtemp())
import zenko as Z  # noqa: E402

HERE = os.path.dirname(__file__)
data = {"VOX_JP": Z.VOX_JP, "VOICE_FOR": Z.VOICE_FOR, "JP_TICS": Z.JP_TICS, "PIPER_WORKER": Z.PIPER_WORKER,
        "TTS_WORKER": Z.TTS_WORKER, "WINSPEECH_WORKER": Z.WINSPEECH_WORKER, "WHISPER_WORKER": Z.WHISPER_WORKER,
        "MED_PROMPT": Z.MED_PROMPT, "PIPER_VOICES": Z.PIPER_VOICES, "HOTKEYS": Z.HOTKEYS,
        "STT_MODELS": Z.STT_MODELS, "STT_PROMPT": Z.STT_PROMPT}
with open(os.path.join(HERE, "..", "crates", "zenko-desktop", "data", "voice.json"), "w", encoding="utf-8") as f:
    json.dump(data, f, ensure_ascii=False, separators=(",", ":"))

TEXTS = ["Kon kon~ ehehe!", "Kon!", "Yatta~! You did it!", "*wags tail* Let's focus on Biology now. You have 25 minutes left!",
         "Here's the link: https://example.com/page and the formula $E=mc^2$ ok?", "**Bold** and _italic_ with `code` # head",
         "Mou~ hmph!", "Fufu, sugoi ne~", "Ganbatte! 🦊✨ You can do it.", "What do you want to study?",
         "I'm so tired and sad today", "Well done! Proud of you.", "Back to work, close YouTube.", "Ehehe kawaii",
         "Mata ne~ oyasumi!", "こんにちは！元気？", "A " * 120 + "end.", "First sentence. " * 20,
         "Line one\nLine two\n- bullet item\n1. numbered", "> quote | table | cell", "Hmm... okay?!", "", "   ",
         "Nee~ what's next?", "Itadakimasu! nom nom", "Gyuu~ *hugs you tight*", "uuu... auu", "kyuun kyuun",
         "Arigatou! Konnichiwa! Ohayou!", "Take a 5 minute break, then 25 more. [Start](cmd:focus)",
         "The answer is \\(x^2\\) and {curly} [square] stuff~ ***triple***", "Hello? Are you there? Hello?"]
g = {"clean": [Z.jp_clean(t) for t in TEXTS], "tics": [Z.jp_tics_only(Z.jp_clean(t)) for t in TEXTS if Z.jp_clean(t)],
     "say": [Z.vv_say_text(t) for t in TEXTS]}
# jp_fallback: the fixed answers, and for the random ones the choice list (random.choice is monkeypatched)
fb = []
for t in TEXTS:
    seen = []
    orig = random.choice
    random.choice = lambda seq: (seen.append(list(seq)), seq[0])[1]
    try:
        r = Z.jp_fallback(t)
    finally:
        random.choice = orig
    fb.append({"r": r, "choices": seen[0] if seen else None})
g["fallback"] = fb
# the voice detector: frames of made-up audio, auto_stop on and off
def frames(levels):
    out = []
    for k, lv in enumerate(levels):
        a = [int(lv * math.sin(i * 0.3 + k)) for i in range(1600)]
        out.append(struct.pack("<%dh" % len(a), *a))
    return out
VADS = {"speech": [50] * 3 + [60] * 5 + [3000] * 10 + [70] * 14, "silent": [40] * 85, "noise": [30, 900, 20] + [400] * 30,
        "short": [100] * 3 + [5000, 5000] + [100] * 20, "long": [50] * 3 + [4000] * 420}
vad = {}
for name, lv in VADS.items():
    for auto in (False, True):
        r = Z.MicRecorder(auto_stop=auto)
        st = {"n": 0, "floor": 0.0, "loud": 0, "quiet": 0}
        res = []
        for fr in frames(lv):
            res.append(bool(r._vad(fr, st)))
        vad["%s_%d" % (name, auto)] = {"levels": lv, "res": res, "heard": r.heard, "st": st}
g["vad"] = vad
g["peak"] = [Z.mic_peak(f) for f in frames([0, 10, 3000, 32767])] + [Z.mic_peak(b""), Z.mic_peak(b"\x01")]
p = os.path.join(tempfile.mkdtemp(), "a.wav")
Z.pcm_to_wav(b"\x01\x02\x03\x04", p)
g["wav"] = list(open(p, "rb").read())
# whisper's output cleaned (stt_transcribe's text handling)
OUTS = ["whisper_init: loading\n Hello there. \nmain: done\n", " [BLANK_AUDIO]\n", "(music) Lock in for 25 minutes (laughs)\n",
        "system_info: x\n  Open   the   timetable  \n\n", "[00:00.000 --> 00:02.000] hi"]
def clean(out):
    text = " ".join(ln.strip() for ln in out.splitlines() if ln.strip() and not ln.startswith(("whisper_", "main:", "system_info")))
    text = re.sub(r"\[[^\]]*\]|\((?:music|noise|silence|inaudible|blank_audio|laughs?|applause)[^)]*\)", " ", text, flags=re.I)
    return re.sub(r"\s+", " ", text).strip()
g["stt_out"] = [[o, clean(o)] for o in OUTS]
# stt_paths / voicevox_find_engine on made-up folders
root = tempfile.mkdtemp()
def touch(rel, size=10):
    pth = os.path.join(root, *rel.split("/"))
    os.makedirs(os.path.dirname(pth), exist_ok=True)
    with open(pth, "wb") as f:
        f.write(b"x" * size)
for rel, sz in [("speech/bin/Release/whisper-cli.exe", 5), ("speech/ggml-small.en-q5_1.bin", 50), ("speech/ggml-tiny.bin", 90),
                ("speech/deep/a/b/c/d/main.exe", 5), ("extra/main.exe", 5), ("extra/whisper.dll", 5),
                ("extra/ggml-base.en-q5_1.bin", 20)]:
    touch(rel, sz)
Z.STT_DIR = os.path.join(root, "speech")
rel = lambda v: None if v is None else os.path.relpath(v, root).replace(os.sep, "/")
g["stt_paths"] = [[k, d, [rel(x) for x in Z.stt_paths(k, os.path.join(root, d) if d else "")]] for k in ("fast", "accurate")
                  for d in ("", "extra")]
eng = tempfile.mkdtemp()
for relp in ["voicevox_engine/windows-cpu/run.exe", "voicevox_engine/run.exe", "Downloads/VOICEVOX-0.20/vv-engine/run.exe",
             "Downloads/voicevox_cpu/a/b/c/d/e/run.exe", "Desktop/MyVoicevox/x/run.exe"]:
    pth = os.path.join(eng, *relp.split("/"))
    os.makedirs(os.path.dirname(pth), exist_ok=True)
    open(pth, "w").close()
import os.path as osp
_eu = osp.expanduser
osp.expanduser = lambda p: eng if p == "~" else _eu(p)
os.environ["LOCALAPPDATA"] = os.path.join(eng, "LocalAppData")
os.environ["ProgramFiles"] = os.path.join(eng, "Program Files")
Z.zenko_app_dir = lambda: os.path.join(eng, "Apps", "Zenko")
r1 = Z.voicevox_find_engine("")
r2 = Z.voicevox_find_engine(os.path.join(eng, "Desktop/MyVoicevox/x/run.exe"))
import shutil
shutil.rmtree(os.path.join(eng, "voicevox_engine"))
r3 = Z.voicevox_find_engine("")
g["engine"] = [os.path.relpath(r, eng).replace(os.sep, "/") if r else "" for r in (r1, r2, r3)]
with open(os.path.join(HERE, "..", "golden", "voice.json"), "w", encoding="utf-8") as f:
    json.dump(g, f, ensure_ascii=False, indent=0)
print("ok", len(TEXTS), g["engine"], g["stt_paths"])
