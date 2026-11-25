// Shared look + settings for every Zenko Browser page (frame, new tab, settings).
// Settings live in localStorage of zenko.localhost, so all of our pages see the same values and
// the frame updates the moment the settings page changes something.
(function () {
  // The same night-shrine colour schemes as Zenko: night, paper, card, ink, grey, accent, accent-light, gold, glow
  const P = {
    classic: ["#130a0f", "#1a1015", "#25161d", "#fff1e2", "#c1a49a", "#ff7a3d", "#ffad7d", "#f4c25b", "#6fd6ff", "Inari"],
    snow: ["#0b0f23", "#11162f", "#191f42", "#eef1ff", "#98a1cd", "#6fb6ff", "#a4d3ff", "#e9c46a", "#7fd8ff", "Byakko"],
    ember: ["#110607", "#190b0b", "#251212", "#fff0e8", "#c69c90", "#ff5a3a", "#ff9c7d", "#ffcf4d", "#ffb43a", "Ember"],
    fennec: ["#110e08", "#18140c", "#241e13", "#fff6e6", "#bba988", "#e9b45a", "#f6d293", "#f0c96a", "#56e0b4", "Desert"],
    midnight: ["#08071a", "#0e0c23", "#181633", "#f1eeff", "#9c96ca", "#a28cff", "#c9bcff", "#e3b04b", "#b9a6ff", "Midnight"],
    sakura: ["#140914", "#1b0d1c", "#28152a", "#fff0f7", "#c89cb6", "#ff86bd", "#ffb6d8", "#f3c56b", "#ff9fd0", "Sakura"],
    mint: ["#05110d", "#0a1914", "#11261e", "#ecfff7", "#90baa9", "#3fd6a4", "#8aeecb", "#e9c46a", "#5ff0c0", "Jade"],
    silver: ["#0c0e13", "#12151c", "#1b1f28", "#f2f5fb", "#9ba4b6", "#8fb0e8", "#bfd2f6", "#d8c08a", "#a8c4ff", "Moonlight"],
    // Night shrine: very dark and low-contrast for late study, warm lantern light and fireflies instead of foxfire
    nightshrine: ["#060509", "#0b0910", "#121019", "#e4dccd", "#8a8190", "#cf9a56", "#e2bb8a", "#c4a062", "#b8e07a", "Night shrine"],
  };
  // Zenko's own subject colours (the same as in Zenko's planner and notebooks)
  const SUBJ = {
    Biology: ["#46b37a", "Bio"], Chemistry: ["#4f8fe0", "Chem"], Maths: ["#e0a33a", "Maths"], French: ["#d65f8f", "French"],
    Psychology: ["#a46ad8", "Psych"], English: ["#e05a4a", "English"], Core: ["#cdb98a", "Core"],
  };
  // The palette in use right now ("Night shrine after 10 pm" swaps it in for the night)
  function palOf(p) {
    const h = new Date().getHours();
    if (p.nightAuto && (h >= 22 || h < 5)) return "nightshrine";
    return P[p.palette] ? p.palette : "snow";
  }
  const FONTS = {
    outfit: ['"Outfit", "Segoe UI Variable Text", "Segoe UI", sans-serif', "Outfit (clean, default)"],
    pixel: ['"Pixelify Sans", "DotGothic16", "Segoe UI", sans-serif', "Pixelify Sans (pixel)"],
    dot: ['"DotGothic16", "Pixelify Sans", "Segoe UI", sans-serif', "DotGothic16 (pixel, Japanese feel)"],
    zen: ['"Zen Maru Gothic", "Yu Gothic UI", "Segoe UI", sans-serif', "Zen Maru Gothic (like Zenko)"],
    segoe: ['"Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif', "Segoe UI"],
    rounded: ['"Nunito", "Varela Round", "Segoe UI Variable Display", "Segoe UI", sans-serif', "Soft rounded"],
    bahn: ['Bahnschrift, "Segoe UI", sans-serif', "Bahnschrift"],
    yu: ['"Yu Gothic UI", "Meiryo UI", "Segoe UI", sans-serif', "Yu Gothic (Japanese)"],
    mono: ['"Cascadia Code", Consolas, monospace', "Cascadia Code"],
    serif: ['Georgia, "Yu Mincho", serif', "Georgia"],
    comic: ['"Comic Sans MS", "Segoe Print", sans-serif', "Comic Sans"],
  };
  // Key names: C = Ctrl, S = Shift, A = Alt, then the Windows key code
  const KEYS = {
    newTab: ["C+84", "New tab"],
    closeTab: ["C+87", "Close tab"],
    reopen: ["C+S+84", "Reopen closed tab"],
    nextTab: ["C+9", "Next tab"],
    prevTab: ["C+S+9", "Previous tab"],
    palette: ["C+75", "Command bar"],
    address: ["C+76", "Address bar"],
    address2: ["A+68", "Address bar (2nd key)"],
    claude: ["C+74", "Claude panel"],
    sideTabs: ["C+S+83", "Side / top tabs"],
    mute: ["C+77", "Mute tab"],
    settings: ["C+188", "Settings"],
    tab1: ["C+49", "Tab 1"], tab2: ["C+50", "Tab 2"], tab3: ["C+51", "Tab 3"], tab4: ["C+52", "Tab 4"],
    tab5: ["C+53", "Tab 5"], tab6: ["C+54", "Tab 6"], tab7: ["C+55", "Tab 7"], tab8: ["C+56", "Tab 8"],
    lastTab: ["C+57", "Last tab"],
    hints: ["A+70", "Link hints (click with the keyboard)"],
    split: ["A+83", "Split view"],
    focus: ["C+S+70", "Focus mode"],
    notes: ["C+S+78", "Notes for this page"],
    pip: ["A+80", "Floating video"],
    clip: ["C+S+88", "Clip part of the page to Zenko"],
    space1: ["A+49", "Space 1"], space2: ["A+50", "Space 2"], space3: ["A+51", "Space 3"],
  };
  const DEFAULTS = {
    palette: "snow", accent: "", font: "outfit", pixel: "subtle", tabShape: "ema", tabs: "top", radius: 12, density: "normal",
    stars: true, motion: "full", glow: true, sleepAfter: 20, engine: "google", panelWidth: 430,
    school: "", keys: {}, uiScale: 100,
    spaces: ["Study", "School", "Fun"], profiles: ["School"],
    ads: true, adsOff: [], clean: true, cookies: true, ytAds: true, sponsor: true, noAutoplay: false, preload: true,
    focus: false, focusBlock: ["youtube.com", "instagram.com", "tiktok.com", "reddit.com", "x.com", "twitter.com", "facebook.com", "netflix.com", "twitch.tv", "discord.com"],
    focusOnlyMode: false, focusOnly: [], focusAllow: ["claude.ai", "inthinking.net", "ibo.org", "khanacademy.org", "wikipedia.org"],
    sortDownloads: true, gestures: true, drafts: true, archiveDays: 7, sites: {}, outlookSound: true, battery: "auto",
    autoGroups: true, groupsFolded: [], foxLife: "discreet", focusWithZenko: true, nightAuto: false,
    translateTo: "en", startWeekday: [], startWeekend: [], weatherCity: "", weatherLat: null, weatherLon: null, schedule: "",
  };
  const ENGINES = {
    google: ["https://www.google.com/search?q=", "Google"],
    bing: ["https://www.bing.com/search?q=", "Bing"],
    ddg: ["https://duckduckgo.com/?q=", "DuckDuckGo"],
  };
  function load() {
    let p = {};
    try { p = JSON.parse(localStorage.getItem("zb.prefs") || "{}") || {}; } catch (e) { p = {}; }
    return Object.assign({}, DEFAULTS, p);
  }
  function save(p) {
    try { localStorage.setItem("zb.prefs", JSON.stringify(p)); } catch (e) {}
  }
  function mix(a, b, t) {
    const pa = parseInt(a.slice(1), 16), pb = parseInt(b.slice(1), 16);
    const ch = (s) => Math.round(((pa >> s) & 255) * (1 - t) + ((pb >> s) & 255) * t);
    return "#" + ((1 << 24) | (ch(16) << 16) | (ch(8) << 8) | ch(0)).toString(16).slice(1);
  }
  function apply(p, root) {
    root = root || document.documentElement;
    const pal = palOf(p), c = P[pal];
    const acc = /^#[0-9a-f]{6}$/i.test(p.accent || "") ? p.accent : c[5];
    const v = {
      "--night": c[0], "--paper": c[1], "--card": c[2], "--ink": c[3], "--grey": c[4], "--acc": acc,
      "--accl": p.accent ? mix(acc, "#ffffff", 0.35) : c[6], "--gold": c[7], "--glow": c[8],
      "--night2": mix(c[0], acc, 0.14), "--field": mix(c[2], c[3], 0.07), "--line": mix(c[2], c[3], 0.14),
      "--edge": mix(c[2], c[3], 0.28), "--hover": mix(c[2], c[3], 0.10), "--accbg": mix(c[2], acc, 0.22),
      "--accdim": mix(c[1], acc, 0.35), "--font": (FONTS[p.font] || FONTS.outfit)[0], "--r": (p.radius | 0) + "px",
      "--lacq": mix(mix(c[0], "#b5362a", 0.55), acc, 0.08), "--lacqd": mix(c[0], "#7a1f18", 0.6),
      "--dusk": mix(c[0], "#7a3d6a", 0.42), "--panel": mix(c[1], c[3], 0.025), "--goldl": mix(c[7], c[2], 0.55),
      "--rs": Math.max(2, Math.round((p.radius | 0) * 0.6)) + "px",
    };
    for (const k in v) root.style.setProperty(k, v[k]);
    root.dataset.palette = pal;
    root.dataset.shape = p.tabShape;
    root.dataset.pixel = p.pixel || "subtle";
    root.dataset.tabs = p.tabs;
    root.dataset.density = p.density;
    root.dataset.motion = p.motion;
    root.dataset.glow = p.glow ? "on" : "off";
    root.dataset.stars = p.stars ? "on" : "off";
    root.style.fontSize = (p.uiScale || 100) * 0.14 + "px";
  }
  function keymap(p) {
    const m = {};
    for (const a in KEYS) m[a] = (p.keys && p.keys[a] !== undefined) ? p.keys[a] : KEYS[a][0];
    return m;
  }
  const VKNAME = { 9: "Tab", 13: "Enter", 27: "Esc", 32: "Space", 37: "Left", 38: "Up", 39: "Right", 40: "Down",
    186: ";", 187: "=", 188: ",", 189: "-", 190: ".", 191: "/", 192: "`", 219: "[", 220: "\\", 221: "]", 222: "'" };
  function keyName(k) {
    if (!k) return "none";
    return k.split("+").map((x) => {
      if (x === "C") return "Ctrl"; if (x === "S") return "Shift"; if (x === "A") return "Alt";
      const n = +x;
      if (VKNAME[n]) return VKNAME[n];
      if (n >= 112 && n <= 123) return "F" + (n - 111);
      return String.fromCharCode(n);
    }).join("+");
  }
  function comboOf(e) {
    if ([16, 17, 18].includes(e.keyCode)) return "";
    return (e.ctrlKey ? "C+" : "") + (e.shiftKey ? "S+" : "") + (e.altKey ? "A+" : "") + e.keyCode;
  }
  // Turn whatever was typed in the address bar into a web address.
  function toUrl(text, p) {
    const t = (text || "").trim();
    if (!t) return "";
    if (/^(https?|file|zenko|about|edge):/i.test(t)) return t;
    if (/^localhost(:\d+)?(\/|$)/i.test(t) || /^\d{1,3}(\.\d{1,3}){3}(:\d+)?(\/|$)/.test(t)) return "http://" + t;
    if (!/\s/.test(t) && /^[^\s\/]+\.[a-z]{2,}(:\d+)?(\/.*)?$/i.test(t)) return "https://" + t;
    const e = ENGINES[(p || load()).engine] || ENGINES.google;
    return e[0] + encodeURIComponent(t);
  }
  // Small safe calculator for the command bar ("12*7+3", "sqrt(2)", "2^10", "15% of 80").
  function calc(text) {
    let s = (text || "").trim().toLowerCase();
    if (!s || !/[\d)]/.test(s) || !/[-+*/^%(]|sqrt|sin|cos|tan|log|ln|pi/.test(s)) return null;
    s = s.replace(/^=/, "").replace(/(\d+(?:\.\d+)?)\s*% of\s*/g, "($1/100)*").replace(/×/g, "*").replace(/÷/g, "/");
    if (!/^[\d\s.+\-*/^%()a-z,]*$/.test(s)) return null;
    let i = 0;
    const peek = () => s[i], ws = () => { while (s[i] === " ") i++; };
    const F = { sqrt: Math.sqrt, sin: Math.sin, cos: Math.cos, tan: Math.tan, log: Math.log10, ln: Math.log, abs: Math.abs };
    function prim() {
      ws();
      if (peek() === "(") { i++; const v = expr(); ws(); if (s[i] !== ")") throw 0; i++; return v; }
      if (peek() === "-") { i++; return -prim(); }
      const m = /^[a-z]+/.exec(s.slice(i));
      if (m) {
        i += m[0].length;
        if (m[0] === "pi") return Math.PI;
        if (m[0] === "e") return Math.E;
        if (!F[m[0]]) throw 0;
        return F[m[0]](prim());
      }
      const n = /^\d*\.?\d+(e[-+]?\d+)?/.exec(s.slice(i));
      if (!n) throw 0;
      i += n[0].length;
      return parseFloat(n[0]);
    }
    function pow() { let b = prim(); ws(); if (peek() === "^") { i++; return Math.pow(b, pow()); } return b; }
    function term() {
      let v = pow();
      for (;;) {
        ws();
        const o = peek();
        if (o === "*") { i++; v *= pow(); } else if (o === "/") { i++; v /= pow(); }
        else if (o === "%") { i++; v %= pow(); } else return v;
      }
    }
    function expr() {
      let v = term();
      for (;;) { ws(); const o = peek(); if (o === "+") { i++; v += term(); } else if (o === "-") { i++; v -= term(); } else return v; }
    }
    try {
      const v = expr(); ws();
      if (i !== s.length || !isFinite(v)) return null;
      return +v.toPrecision(12);
    } catch (e) { return null; }
  }
  const SITES = [
    ["Google", "https://www.google.com/", "G"], ["Outlook", "https://outlook.office.com/mail/", "O"],
    ["Claude", "https://claude.ai/new", "C"], ["YouTube", "https://www.youtube.com/", "Y"],
    ["InThinking", "https://www.inthinking.net/", "i"], ["GitHub", "https://github.com/", "H"],
  ];
  // "5 km to mi", "100 f in c", "2.5 kg to lb"
  const U = {
    km: ["l", 1000], m: ["l", 1], cm: ["l", 0.01], mm: ["l", 0.001], mi: ["l", 1609.344], mile: ["l", 1609.344], miles: ["l", 1609.344], ft: ["l", 0.3048], feet: ["l", 0.3048], in: ["l", 0.0254], inch: ["l", 0.0254], yd: ["l", 0.9144],
    kg: ["w", 1], g: ["w", 0.001], mg: ["w", 1e-6], lb: ["w", 0.45359237], lbs: ["w", 0.45359237], oz: ["w", 0.028349523],
    l: ["v", 1], ml: ["v", 0.001], cl: ["v", 0.01], gal: ["v", 3.785411784], cup: ["v", 0.2365882],
    s: ["t", 1], sec: ["t", 1], min: ["t", 60], h: ["t", 3600], hr: ["t", 3600], hours: ["t", 3600], day: ["t", 86400], days: ["t", 86400], week: ["t", 604800],
    kmh: ["s", 1 / 3.6], "km/h": ["s", 1 / 3.6], mph: ["s", 0.44704], "m/s": ["s", 1],
    j: ["e", 1], kj: ["e", 1000], cal: ["e", 4.184], kcal: ["e", 4184], ev: ["e", 1.602176634e-19],
    pa: ["p", 1], kpa: ["p", 1000], atm: ["p", 101325], bar: ["p", 1e5], mmhg: ["p", 133.322],
  };
  function units(text) {
    const m = (text || "").trim().toLowerCase().match(/^(-?[\d.]+)\s*([a-z°\/]+)\s+(?:to|in|as)\s+([a-z°\/]+)$/);
    if (!m) return null;
    const v = parseFloat(m[1]);
    let a = m[2].replace("°", ""), b = m[3].replace("°", "");
    const T = { c: (x) => x, f: (x) => (x - 32) * 5 / 9, k: (x) => x - 273.15 }, TB = { c: (x) => x, f: (x) => x * 9 / 5 + 32, k: (x) => x + 273.15 };
    if (T[a] && TB[b]) return { v: TB[b](T[a](v)), u: b.toUpperCase() === "K" ? "K" : "°" + b.toUpperCase() };
    if (!U[a] || !U[b] || U[a][0] !== U[b][0]) return null;
    return { v: v * U[a][1] / U[b][1], u: m[3] };
  }
  function sites(p) {
    const s = SITES.slice();
    if (p.school) s.push(["School", toUrl(p.school, p), "S"]);
    return s;
  }
  // Pixel icons: strings of rows, "#" = filled pixel. Drawn as crisp SVG squares.
  const ICONS = {
    shield: ["#######", "#.....#", "#.###.#", "#.###.#", ".#.#.#.", "..#.#..", "...#..."],
    note: ["######.", "#....##", "#.##..#", "#.....#", "#.###.#", "#.....#", "#######"],
    down: ["...#...", "...#...", "...#...", "#.###.#", ".#####.", "..###..", "#######"],
    clock: ["..###..", ".#...#.", "#..#..#", "#..##.#", "#.....#", ".#...#.", "..###.."],
    music: ["..#####", "..#...#", "..#...#", "..#...#", "###.###", "###.###", "###.###"],
    split: ["###.###", "#.#.#.#", "#.#.#.#", "#.#.#.#", "#.#.#.#", "#.#.#.#", "###.###"],
    clip: ["#.....#", ".#...#.", "..#.#..", "...#...", "..#.#..", "##...##", "##...##"],
    book: ["##.##..", "#.#.#..", "#.#.#..", "#.#.#..", "#.#.#..", "##.##..", "......."],
    globe: ["..###..", ".#.#.#.", "#..#..#", "#######", "#..#..#", ".#.#.#.", "..###.."],
    save: ["######.", "#.##.##", "#.##..#", "#.....#", "#.###.#", "#.###.#", "#######"],
    eye: [".......", "..###..", ".#...#.", "#..#..#", ".#...#.", "..###..", "......."],
    space: ["##.##..", "##.##..", ".......", "##.##..", "##.##..", ".......", "......."],
    user: ["..###..", "..###..", "..###..", ".......", ".#####.", "#######", "#######"],
    bolt: ["....##.", "...##..", "..##...", ".######", "...##..", "..##...", ".##...."],
    keyb: ["#######", "#.#.#.#", "#######", "#.#.#.#", "#######", "#.....#", "#######"],
    back: ["...#....", "..##....", ".#######", "########", ".#######", "..##....", "...#...."],
    fwd: ["....#...", "....##..", "#######.", "########", "#######.", "....##..", "....#..."],
    reload: ["..####..", ".#....#.", "#.......", "#.....##", "#....###", ".#....#.", "..####.."],
    stop: ["#.....#", ".#...#.", "..#.#..", "...#...", "..#.#..", ".#...#.", "#.....#"],
    plus: ["...#...", "...#...", "...#...", "#######", "...#...", "...#...", "...#..."],
    close: ["#....#", ".#..#.", "..##..", "..##..", ".#..#.", "#....#"],
    min: ["......", "......", "......", "......", "......", "######"],
    max: ["######", "#....#", "#....#", "#....#", "#....#", "######"],
    rest: ["..####", "..#..#", "######", "#...##", "#...#.", "#####."],
    menu: ["##.##.##"],
    claude: ["...#...", "#..#..#", ".#.#.#.", "..###..", ".#.#.#.", "#..#..#", "...#..."],
    side: ["########", "#..#....", "#..#....", "#..#....", "#..#....", "########"],
    pop: ["...####", "....###", "...#.##", "..#...#", ".#.....", "#......"],
    home: ["...##...", "..####..", ".######.", "########", ".##..##.", ".##..##."],
    sound: ["...#....", "..##..#.", "####.#..", "####.#.#", "####.#..", "..##..#.", "...#...."],
    muted: ["...#....", "..##....", "####.#.#", "####..#.", "####.#.#", "..##....", "...#...."],
    moon: ["..###..", ".##....", "##.....", "##.....", "##.....", ".##....", "..###.."],
    search: [".###...", "#...#..", "#...#..", "#...#..", ".###...", "....##.", ".....##"],
    gear: ["..#.#..", ".#####.", "##...##", ".#...#.", "##...##", ".#####.", "..#.#.."],
    zzz: ["####", "..#.", ".#..", "####"],
    lock: [".###.", "#...#", "#...#", "#####", "##.##", "#####"],
    mask: ["w.......w", "ww.....ww", "wrw...wrw", "wwwwwwwww", "wwrwwwrww", "wd.wgw.dw", "wwwwwwwww", ".wwrwrww.", "..wwwww..", "...wgw..."],
    torii: ["rrrrrrrrr", ".RrrrrrR.", "..r...r..", "rrrrrrrrr", "..r...r..", "..r...r..", "..r...r..", ".RR...RR."],
    moon2: ["..gggg.", ".gg..g.", "gg.....", "gg.....", "gg.....", ".gg..g.", "..gggg."],
    diamond: ["..g..", ".ggg.", "ggggg", ".ggg.", "..g.."],
    claudec: ["...a...", "a..a..a", ".a.a.a.", "..aaa..", ".a.a.a.", "a..a..a", "...a..."],
    fox: ["w.....w", "ww...ww", "wwwwwww", "w.www.w", "wwwwwww", ".wwgww.", "..www.."],
    star: ["...#...", "..###..", "#######", ".#####.", "..###..", ".##.##.", "#.....#"],
  };
  function icon(name, px) {
    const rows = ICONS[name] || ICONS.star;
    const h = rows.length, w = Math.max(...rows.map((r) => r.length));
    let r = "";
    const COL = { g: "var(--gold)", r: "#e0533a", R: "#a8321f", p: "#ff9fc6", P: "#f06d9f", b: "#8a5a3c", w: "var(--ink)", a: "var(--acc)", d: "var(--edge)", s: "#c8cfe8", f: "var(--glow)" };
    rows.forEach((row, y) => { for (let x = 0; x < row.length; x++) { const ch = row[x]; if (ch === "." || ch === " ") continue;
      r += `<rect x="${x}" y="${y}" width="1.02" height="1.02"${ch === "#" ? "" : ` fill="${COL[ch] || "currentColor"}"`}/>`; } });
    const s = px || 2;
    return `<svg class="px" viewBox="0 0 ${w} ${h}" width="${w * s}" height="${h * s}" shape-rendering="crispEdges" fill="currentColor">${r}</svg>`;
  }
  // Slow twinkling pixel stars on a canvas (about 12 frames a second, pauses when hidden).
  function starfield(cv) {
    if (!cv) return;
    const ctx = cv.getContext("2d");
    let stars = [], w = 0, h = 0, last = 0;
    function size() {
      const s = 3; // one star pixel = 3 css px, crisp
      w = Math.ceil(cv.clientWidth / s); h = Math.ceil(cv.clientHeight / s);
      cv.width = w; cv.height = h;
      const n = Math.round(w * h / 900);
      stars = [];
      for (let i = 0; i < n; i++) stars.push({ x: Math.random() * w | 0, y: Math.random() * h * 0.9 | 0, p: Math.random() * 6.28, s: 0.4 + Math.random() * 1.2, big: Math.random() < 0.08 });
    }
    function frame(t) {
      requestAnimationFrame(frame);
      if (t - last < 80 || document.hidden) return;
      last = t;
      const css = getComputedStyle(document.documentElement);
      const ink = css.getPropertyValue("--ink").trim() || "#fff", glow = css.getPropertyValue("--glow").trim() || "#7fd8ff";
      ctx.clearRect(0, 0, w, h);
      for (const s of stars) {
        const a = 0.25 + 0.55 * (0.5 + 0.5 * Math.sin(t / 1000 * s.s + s.p));
        ctx.globalAlpha = a * (s.y < h * 0.6 ? 1 : 0.5);
        ctx.fillStyle = s.big ? glow : ink;
        ctx.fillRect(s.x, s.y, 1, 1);
        if (s.big && a > 0.6) { ctx.globalAlpha = a * 0.35; ctx.fillRect(s.x - 1, s.y, 3, 1); ctx.fillRect(s.x, s.y - 1, 1, 3); }
      }
    }
    size();
    new ResizeObserver(size).observe(cv);
    requestAnimationFrame(frame);
  }
  // Seigaiha (blue-ocean waves) like Zenko's banner, drawn once into a canvas.
  function seigaiha(cv, bg, line, k) {
    k = document.documentElement.dataset.pixel === "strong" ? (k || 0.5) : (window.devicePixelRatio || 1) * 1.0;
    const w = cv.width = Math.ceil((cv.clientWidth || 400) * k), h = cv.height = Math.ceil((cv.clientHeight || 60) * k);
    const x = cv.getContext("2d"), R = Math.max(6, Math.round(13 * k * 2));
    x.fillStyle = bg; x.fillRect(0, 0, w, h);
    x.lineWidth = k >= 1 ? 1.2 * k : 1;
    for (let row = -2, y = -R; y < h + R; row++, y += R / 2) {
      const off = (row & 1) ? R : 0;
      for (let cx = -R * 2 + off; cx < w + R * 2; cx += R * 2) {
        x.fillStyle = bg; x.beginPath(); x.arc(cx, y, R, Math.PI, 0); x.lineTo(cx + R, y + R); x.lineTo(cx - R, y + R); x.fill();
        for (const k of [1, 0.76, 0.52, 0.28]) { x.strokeStyle = line; x.beginPath(); x.arc(cx, y + 0.5, R * k, Math.PI, 0); x.stroke(); }
      }
    }
  }
  // Pixel landscape: dusk hills, a torii, lanterns with foxfire. Returns SVG markup (viewBox 0 0 W H).
  function landscape(W, H, opt) {
    opt = opt || {};
    const css = getComputedStyle(document.documentElement), c = (v) => css.getPropertyValue(v).trim();
    let r = "";
    const box = (x, y, w, h, f, o) => { r += `<rect x="${x}" y="${y}" width="${w}" height="${h}" fill="${f}"${o ? ` opacity="${o}"` : ""}/>`; };
    const far = mix(c("--dusk"), c("--night"), 0.25), near = mix(c("--night"), "#000000", 0.25);
    for (let x = 0; x < W; x += 2) { const h = Math.round(H * 0.42 + H * 0.14 * Math.sin(x / (W / 5.5) + 1) + H * 0.05 * Math.sin(x / 7)); box(x, H - h, 2, h, far); }
    for (let x = 0; x < W; x += 2) { const h = Math.round(H * 0.22 + H * 0.08 * Math.sin(x / (W / 4) + 3) + H * 0.03 * Math.sin(x / 5)); box(x, H - h, 2, h, near); }
    const tx = Math.round(opt.toriiX != null ? opt.toriiX : W * 0.22), ty = Math.round(H * 0.38), red = "#d9472b", dr = "#8f2a1a";
    box(tx - 4, ty, 36, 3, red); box(tx - 6, ty - 1, 4, 2, red); box(tx + 30, ty - 1, 4, 2, red); box(tx - 2, ty + 3, 32, 1, dr);
    box(tx + 1, ty + 7, 26, 2, red); box(tx + 12, ty + 3, 4, 4, red);
    box(tx + 3, ty + 4, 3, H - ty - 4, red); box(tx + 22, ty + 4, 3, H - ty - 4, red); box(tx + 3, ty + 4, 1, H - ty - 4, dr); box(tx + 22, ty + 4, 1, H - ty - 4, dr);
    (opt.lights || [0.62, 0.7]).forEach((f) => { const lx = Math.round(W * f), ly = Math.round(H * 0.72); box(lx - 2, ly - 2, 7, 7, c("--gold"), 0.18); box(lx, ly, 3, 3, c("--gold")); });
    return r;
  }
  // The animated night sky behind the browser frame and on the new-tab page.
  // "subtle" pixel flair: drawn at full screen resolution (crisp), with stars, foxfire and hills built from tiny
  // 2px squares. "strong": the old chunky look (1/3 resolution, dithered). Pauses when hidden.
  function sky(cv, opt) {
    if (!cv) return () => {};
    opt = Object.assign({ moon: [0.8, 0.16, 6], ridge: 0, torii: false, wisps: 5, shoot: true, fps: 24 }, opt || {});
    const ctx = cv.getContext("2d");
    const base = document.createElement("canvas"), bx = base.getContext("2d");
    let w = 0, h = 0, u = 1, ff = false, stars = [], wisps = [], shot = null, nextShot = 0, last = 0, col = {}, strong = false;
    const B4 = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
    function colors() {
      const cs = getComputedStyle(document.documentElement), v = (k) => cs.getPropertyValue(k).trim() || "#000000";
      col = { top: v("--night2"), mid: v("--night"), low: mix(v("--night"), v("--acc"), 0.14), dusk: v("--dusk"),
        ink: v("--ink"), glow: v("--glow"), gold: v("--gold"), moon: "#f6e7bd", moonD: "#d8c08a",
        ridge: mix(v("--night"), "#000000", 0.35), ridge2: mix(v("--night"), v("--dusk"), 0.35) };
    }
    function build(o) {
      if (o && typeof o === "object" && !Array.isArray(o)) Object.assign(opt, o);
      colors();
      strong = document.documentElement.dataset.pixel === "strong";
      const dpr = window.devicePixelRatio || 1;
      const k = strong ? 1 / 3 : dpr;                     // canvas pixels per css px
      u = strong ? 1 : Math.max(1, Math.round(2 * dpr));  // one "art pixel" in canvas pixels
      w = Math.max(8, Math.ceil(cv.clientWidth * k)); h = Math.max(8, Math.ceil(cv.clientHeight * k));
      cv.width = w; cv.height = h; base.width = w; base.height = h;
      cv.style.imageRendering = strong ? "pixelated" : "auto";
      const g = bx.createLinearGradient(0, 0, 0, h);
      g.addColorStop(0, col.top); g.addColorStop(0.55, col.mid); g.addColorStop(1, opt.ridge ? col.dusk : col.low);
      bx.fillStyle = g; bx.fillRect(0, 0, w, h);
      if (strong) {   // ordered dithering bands
        const img = bx.getImageData(0, 0, w, h), d = img.data;
        for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
          const i = (y * w + x) * 4, t = B4[(y & 3) * 4 + (x & 3)] / 16 - 0.5;
          for (let c = 0; c < 3; c++) d[i + c] = Math.max(0, Math.min(255, Math.round((d[i + c] + t * 10) / 6) * 6));
        }
        bx.putImageData(img, 0, 0);
      }
      if (opt.moon) {
        const [mxf, myf, r0] = opt.moon, r = r0 * u * (strong ? 1 : 0.9), mx = w * mxf, my = Math.max(r + 4 * u, h * myf);
        const halo = bx.createRadialGradient(mx, my, r * 0.8, mx, my, r * 4);
        halo.addColorStop(0, "rgba(246,231,189,.22)"); halo.addColorStop(1, "rgba(246,231,189,0)");
        bx.fillStyle = halo; bx.fillRect(mx - r * 4, my - r * 4, r * 8, r * 8);
        const body = bx.createRadialGradient(mx - r * 0.35, my - r * 0.35, r * 0.1, mx, my, r);
        body.addColorStop(0, "#fffaf0"); body.addColorStop(0.6, col.moon); body.addColorStop(1, col.moonD);
        bx.fillStyle = body; bx.beginPath(); bx.arc(mx, my, r, 0, 6.2832); bx.fill();
        bx.fillStyle = "rgba(170,140,90,.22)";
        [[-0.3, -0.1, 0.22], [0.25, 0.3, 0.16], [0.1, -0.4, 0.1]].forEach(([dx, dy, rr]) => { bx.beginPath(); bx.arc(mx + dx * r, my + dy * r, rr * r, 0, 6.2832); bx.fill(); });
      }
      if (opt.ridge) {
        const rh = h * opt.ridge;
        for (let x = 0; x < w; x += u) {
          const X = x / u, a = Math.round(rh * (0.62 + 0.25 * Math.sin(X / 23 + 1.3) + 0.1 * Math.sin(X / 7)) / u) * u;
          bx.fillStyle = col.ridge2; bx.fillRect(x, h - a, u, a);
          const b = Math.round(rh * (0.32 + 0.12 * Math.sin(X / 31 + 4) + 0.05 * Math.sin(X / 5)) / u) * u;
          bx.fillStyle = col.ridge; bx.fillRect(x, h - b, u, b);
        }
        if (opt.torii) {
          const P = (x, y, ww, hh, c) => { bx.fillStyle = c; bx.fillRect(tx + x * u, ty + y * u, ww * u, hh * u); };
          const tx = Math.round(w * opt.torii / u) * u, ty = Math.round((h - rh * 0.62) / u) * u - 10 * u, red = "#d9472b", dr = "#8f2a1a";
          P(-1, 0, 13, 1, red); P(0, 1, 11, 1, dr); P(0, 2, 11, 1, red); P(2, 1, 1, 9, red); P(8, 1, 1, 9, red);
          bx.globalAlpha = 0.9; P(-6, 7, 1, 1, col.gold); P(17, 7, 1, 1, col.gold); bx.globalAlpha = 1;
        }
      }
      stars = [];
      const n = Math.round((w / u) * (h / u) / 110);
      for (let i = 0; i < n; i++) stars.push({ x: Math.round(Math.random() * w / u) * u, y: Math.round(Math.random() * h * (opt.ridge ? 0.7 : 1) / u) * u, p: Math.random() * 6.28, s: 0.5 + Math.random() * 1.6, big: Math.random() < 0.07, g: Math.random() < 0.3 });
      wisps = [];
      ff = document.documentElement.dataset.palette === "nightshrine";
      for (let i = 0; i < (ff ? opt.wisps * 2 + 4 : opt.wisps); i++) wisps.push({ x: Math.random() * w, y: h * (0.3 + Math.random() * 0.65), vx: (Math.random() - 0.5) * 0.12, ph: Math.random() * 6.28, life: Math.random() });
    }
    function frame(t) {
      requestAnimationFrame(frame);
      if (t - last < 1000 / opt.fps || document.hidden || (document.documentElement.dataset.motion === "reduced" && last)) return;
      last = t;
      ctx.drawImage(base, 0, 0);
      const sec = t / 1000;
      if (document.documentElement.dataset.stars !== "off") for (const s of stars) {
        const a = 0.15 + 0.85 * Math.max(0, Math.sin(sec * s.s + s.p));
        ctx.globalAlpha = a; ctx.fillStyle = s.g ? col.glow : col.ink;
        ctx.fillRect(s.x, s.y, u, u);
        if (s.big && a > 0.75) { ctx.globalAlpha = a - 0.6; ctx.fillRect(s.x - u, s.y, 3 * u, u); ctx.fillRect(s.x, s.y - u, u, 3 * u); }
      }
      if (ff) {   // Night shrine: fireflies drift low and blink slowly, green-gold, never bright
        for (const p of wisps) {
          p.ph += 0.018; p.life += 0.0016;
          p.x += (Math.sin(p.ph * 0.7 + p.vx * 40) * 0.12 + p.vx) * u; p.y += Math.cos(p.ph * 0.9) * 0.08 * u;
          if (p.x < -4 * u) p.x = w + 2 * u; if (p.x > w + 4 * u) p.x = -2 * u;
          if (p.y < h * 0.25) p.y = h * 0.25; if (p.y > h - u) p.y = h - u;
          const blink = Math.max(0, Math.sin(p.ph * 2.2 + p.life * 9));
          const a = blink * blink * 0.75;
          if (a < 0.02) continue;
          const x = Math.round(p.x / u) * u, y = Math.round(p.y / u) * u;
          if (!strong) {
            const gl = ctx.createRadialGradient(x, y, 0, x, y, 5 * u);
            gl.addColorStop(0, "rgba(190,230,110,.55)"); gl.addColorStop(1, "rgba(0,0,0,0)");
            ctx.globalAlpha = a * 0.5; ctx.fillStyle = gl; ctx.fillRect(x - 5 * u, y - 5 * u, 10 * u, 10 * u);
          }
          ctx.globalAlpha = a; ctx.fillStyle = "#d8f08a"; ctx.fillRect(x, y, u, u);
        }
      } else for (const p of wisps) {
        p.ph += 0.03; p.x += (p.vx + Math.sin(p.ph) * 0.06) * u; p.y -= 0.05 * u; p.life += 0.002;
        if (p.y < h * 0.15 || p.x < -4 * u || p.x > w + 4 * u || p.life > 1) { p.x = Math.random() * w; p.y = h * (0.55 + Math.random() * 0.4); p.life = 0; }
        const a = Math.sin(p.life * Math.PI) * (0.6 + 0.4 * Math.sin(p.ph * 3));
        const x = Math.round(p.x / u) * u, y = Math.round(p.y / u) * u;
        if (!strong) {
          const gl = ctx.createRadialGradient(x, y, 0, x, y, 6 * u);
          gl.addColorStop(0, col.glow); gl.addColorStop(1, "rgba(0,0,0,0)");
          ctx.globalAlpha = a * 0.35; ctx.fillStyle = gl; ctx.fillRect(x - 6 * u, y - 6 * u, 12 * u, 12 * u);
        } else { ctx.globalAlpha = a * 0.25; ctx.fillStyle = col.glow; ctx.fillRect(x - 2 * u, y - u, 5 * u, 3 * u); ctx.fillRect(x - u, y - 2 * u, 3 * u, 5 * u); }
        ctx.fillStyle = col.glow; ctx.globalAlpha = a * 0.9; ctx.fillRect(x - u, y, 3 * u, u); ctx.fillRect(x, y - u, u, 3 * u);
        ctx.fillStyle = "#ffffff"; ctx.globalAlpha = a; ctx.fillRect(x, y, u, u);
      }
      if (opt.shoot) {
        if (!shot && sec > nextShot) { shot = { x: w * (0.3 + Math.random() * 0.6), y: Math.random() * h * 0.3, k: 0 }; nextShot = sec + 9 + Math.random() * 14; }
        if (shot) {
          shot.k += 1;
          for (let i = 0; i < 12; i++) { ctx.globalAlpha = (1 - i / 12) * Math.max(0, 1 - shot.k / 24); ctx.fillStyle = i < 2 ? "#ffffff" : col.ink; ctx.fillRect(Math.round(shot.x - (shot.k * 2 - i * 2) * u), Math.round(shot.y + (shot.k - i) * u), u, u); }
          if (shot.k > 24) shot = null;
        }
      }
      ctx.globalAlpha = 1;
    }
    build();
    let rt = 0;
    new ResizeObserver(() => { clearTimeout(rt); rt = setTimeout(() => build(), 60); }).observe(cv);
    requestAnimationFrame(frame);
    return build;
  }
  window.ZB = { SUBJ, palOf, sky, starfield, seigaiha, landscape, PALETTES: P, FONTS, KEYS, DEFAULTS, ENGINES, load, save, apply, keymap, keyName, comboOf, toUrl, calc, units, sites, icon, mix };
})();
