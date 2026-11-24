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
  };
  const FONTS = {
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
  };
  const DEFAULTS = {
    palette: "snow", accent: "", font: "pixel", tabShape: "ema", tabs: "top", radius: 12, density: "normal",
    stars: true, motion: "full", glow: true, sleepAfter: 20, engine: "google", panelWidth: 430,
    school: "", keys: {}, uiScale: 100,
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
    const c = P[p.palette] || P.snow;
    const acc = /^#[0-9a-f]{6}$/i.test(p.accent || "") ? p.accent : c[5];
    const v = {
      "--night": c[0], "--paper": c[1], "--card": c[2], "--ink": c[3], "--grey": c[4], "--acc": acc,
      "--accl": p.accent ? mix(acc, "#ffffff", 0.35) : c[6], "--gold": c[7], "--glow": c[8],
      "--night2": mix(c[0], acc, 0.14), "--field": mix(c[2], c[3], 0.07), "--line": mix(c[2], c[3], 0.14),
      "--edge": mix(c[2], c[3], 0.28), "--hover": mix(c[2], c[3], 0.10), "--accbg": mix(c[2], acc, 0.22),
      "--accdim": mix(c[1], acc, 0.35), "--font": (FONTS[p.font] || FONTS.pixel)[0], "--r": (p.radius | 0) + "px",
      "--lacq": mix(mix(c[0], "#b5362a", 0.55), acc, 0.08), "--lacqd": mix(c[0], "#7a1f18", 0.6),
      "--dusk": mix(c[0], "#7a3d6a", 0.42), "--panel": mix(c[1], c[3], 0.025), "--goldl": mix(c[7], c[2], 0.55),
      "--rs": Math.max(2, Math.round((p.radius | 0) * 0.6)) + "px",
    };
    for (const k in v) root.style.setProperty(k, v[k]);
    root.dataset.shape = p.tabShape;
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
  function sites(p) {
    const s = SITES.slice();
    if (p.school) s.push(["School", toUrl(p.school, p), "S"]);
    return s;
  }
  // Pixel icons: strings of rows, "#" = filled pixel. Drawn as crisp SVG squares.
  const ICONS = {
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
    k = k || 0.5;
    const w = cv.width = Math.ceil((cv.clientWidth || 400) * k), h = cv.height = Math.ceil((cv.clientHeight || 60) * k);
    const x = cv.getContext("2d"), R = Math.max(6, Math.round(13 * k * 2));
    x.imageSmoothingEnabled = false;
    x.fillStyle = bg; x.fillRect(0, 0, w, h);
    x.lineWidth = 1;
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
  // The animated pixel night sky used behind the browser frame and on the new-tab page.
  // Drawn at 1/3 resolution and scaled up crisply: dithered gradient, twinkling stars, a pixel moon,
  // drifting foxfire wisps and the odd shooting star. ~20 fps, pauses when hidden.
  function sky(cv, opt) {
    if (!cv) return () => {};
    opt = Object.assign({ S: 3, moon: [0.8, 0.16, 6], ridge: 0, torii: false, wisps: 5, shoot: true, fps: 20 }, opt || {});
    const ctx = cv.getContext("2d");
    const base = document.createElement("canvas"), bx = base.getContext("2d");
    let w = 0, h = 0, stars = [], wisps = [], shot = null, nextShot = 0, last = 0, col = {};
    const B4 = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
    const hex = (c) => { const n = parseInt(c.slice(1), 16); return [(n >> 16) & 255, (n >> 8) & 255, n & 255]; };
    const rgb = (a) => `rgb(${a[0] | 0},${a[1] | 0},${a[2] | 0})`;
    const lerp = (a, b, t) => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
    function colors() {
      const cs = getComputedStyle(document.documentElement), v = (k) => cs.getPropertyValue(k).trim() || "#000000";
      col = { top: hex(v("--night2")), mid: hex(v("--night")), low: hex(mix(v("--night"), v("--acc"), 0.16)), dusk: hex(v("--dusk")),
        ink: v("--ink"), glow: v("--glow"), gold: v("--gold"), acc: v("--acc"), moon: "#f3e2b3", moonD: "#cdb57f",
        ridge: mix(v("--night"), "#000000", 0.35), ridge2: mix(v("--night"), v("--dusk"), 0.35) };
    }
    function build(o) {
      if (o && typeof o === "object" && !Array.isArray(o)) Object.assign(opt, o);
      colors();
      w = Math.max(8, Math.ceil(cv.clientWidth / opt.S)); h = Math.max(8, Math.ceil(cv.clientHeight / opt.S));
      cv.width = w; cv.height = h; base.width = w; base.height = h;
      const img = bx.createImageData(w, h), d = img.data, bands = 9;
      for (let y = 0; y < h; y++) {
        const t = y / h;
        const c = t < 0.55 ? lerp(col.top, col.mid, t / 0.55) : lerp(col.mid, opt.ridge ? col.dusk : col.low, (t - 0.55) / 0.45);
        const c2 = t < 0.55 ? lerp(col.top, col.mid, Math.min(1, (t + 1 / bands) / 0.55)) : lerp(col.mid, opt.ridge ? col.dusk : col.low, Math.min(1, (t + 1 / bands - 0.55) / 0.45));
        const f = (t * bands) % 1;
        for (let x = 0; x < w; x++) {
          const use = (B4[(y & 3) * 4 + (x & 3)] / 16) < f ? c2 : c, i = (y * w + x) * 4;
          d[i] = use[0]; d[i + 1] = use[1]; d[i + 2] = use[2]; d[i + 3] = 255;
        }
      }
      bx.putImageData(img, 0, 0);
      if (opt.moon) {
        const [mxf, myf, r] = opt.moon, mx = Math.round(w * mxf), my = Math.round(Math.max(r + 2, h * myf));
        for (let y = -r - 5; y <= r + 5; y++) for (let x = -r - 5; x <= r + 5; x++) {
          const dd = Math.sqrt(x * x + y * y);
          if (dd <= r) {
            const shade = Math.sqrt((x + 3) * (x + 3) + (y + 2) * (y + 2)) < r - 1 ? col.moon : col.moonD;
            bx.fillStyle = shade; bx.fillRect(mx + x, my + y, 1, 1);
          } else if (dd <= r + 5 && B4[((y + 64) & 3) * 4 + ((x + 64) & 3)] / 16 < 0.32 * (1 - (dd - r) / 5)) {
            bx.fillStyle = col.moonD; bx.globalAlpha = 0.35; bx.fillRect(mx + x, my + y, 1, 1); bx.globalAlpha = 1;
          }
        }
        bx.fillStyle = col.moonD; bx.fillRect(mx - 2, my - 1, 2, 2); bx.fillRect(mx + 2, my + 2, 1, 1);
      }
      if (opt.ridge) {
        const rh = h * opt.ridge;
        for (let x = 0; x < w; x++) {
          const a = Math.round(rh * (0.62 + 0.25 * Math.sin(x / 23 + 1.3) + 0.1 * Math.sin(x / 7)));
          bx.fillStyle = col.ridge2; bx.fillRect(x, h - a, 1, a);
          const b = Math.round(rh * (0.32 + 0.12 * Math.sin(x / 31 + 4) + 0.05 * Math.sin(x / 5)));
          bx.fillStyle = col.ridge; bx.fillRect(x, h - b, 1, b);
        }
        if (opt.torii) {
          const tx = Math.round(w * opt.torii), ty = Math.round(h - rh * 0.62 - 9), red = "#d9472b", dr = "#8f2a1a";
          bx.fillStyle = red; bx.fillRect(tx - 1, ty, 13, 1); bx.fillRect(tx, ty + 2, 11, 1); bx.fillRect(tx + 2, ty + 1, 1, 9); bx.fillRect(tx + 8, ty + 1, 1, 9);
          bx.fillStyle = dr; bx.fillRect(tx, ty + 1, 11, 1);
          bx.fillStyle = col.gold; bx.fillRect(tx - 6, ty + 7, 1, 1); bx.fillRect(tx + 17, ty + 7, 1, 1);
        }
      }
      stars = [];
      const n = Math.round(w * h / 140);
      for (let i = 0; i < n; i++) stars.push({ x: Math.random() * w | 0, y: Math.random() * h * (opt.ridge ? 0.7 : 1) | 0, p: Math.random() * 6.28, s: 0.5 + Math.random() * 1.6, big: Math.random() < 0.07, g: Math.random() < 0.3 });
      wisps = [];
      for (let i = 0; i < opt.wisps; i++) wisps.push({ x: Math.random() * w, y: h * (0.3 + Math.random() * 0.65), vx: (Math.random() - 0.5) * 0.12, ph: Math.random() * 6.28, life: Math.random() });
    }
    function frame(t) {
      requestAnimationFrame(frame);
      if (t - last < 1000 / opt.fps || document.hidden || document.documentElement.dataset.motion === "reduced" && last) return;
      last = t;
      ctx.drawImage(base, 0, 0);
      const sec = t / 1000;
      if (document.documentElement.dataset.stars !== "off") for (const s of stars) {
        const a = 0.2 + 0.8 * Math.max(0, Math.sin(sec * s.s + s.p));
        ctx.globalAlpha = a; ctx.fillStyle = s.g ? col.glow : col.ink;
        ctx.fillRect(s.x, s.y, 1, 1);
        if (s.big && a > 0.75) { ctx.globalAlpha = (a - 0.6); ctx.fillRect(s.x - 1, s.y, 3, 1); ctx.fillRect(s.x, s.y - 1, 1, 3); }
      }
      for (const p of wisps) {
        p.ph += 0.03; p.x += p.vx + Math.sin(p.ph) * 0.06; p.y -= 0.05; p.life += 0.002;
        if (p.y < h * 0.15 || p.x < -4 || p.x > w + 4 || p.life > 1) { p.x = Math.random() * w; p.y = h * (0.55 + Math.random() * 0.4); p.life = 0; }
        const a = Math.sin(p.life * Math.PI) * (0.6 + 0.4 * Math.sin(p.ph * 3));
        const x = Math.round(p.x), y = Math.round(p.y);
        ctx.globalAlpha = a * 0.25; ctx.fillStyle = col.glow; ctx.fillRect(x - 2, y - 1, 5, 3); ctx.fillRect(x - 1, y - 2, 3, 5);
        ctx.globalAlpha = a * 0.9; ctx.fillRect(x - 1, y, 3, 1); ctx.fillRect(x, y - 1, 1, 3);
        ctx.fillStyle = "#ffffff"; ctx.globalAlpha = a; ctx.fillRect(x, y, 1, 1);
      }
      if (opt.shoot) {
        if (!shot && sec > nextShot) { shot = { x: w * (0.3 + Math.random() * 0.6), y: Math.random() * h * 0.3, k: 0 }; nextShot = sec + 9 + Math.random() * 14; }
        if (shot) {
          shot.k += 1;
          for (let i = 0; i < 9; i++) { ctx.globalAlpha = (1 - i / 9) * Math.max(0, 1 - shot.k / 22); ctx.fillStyle = i < 2 ? "#ffffff" : col.ink; ctx.fillRect(Math.round(shot.x - shot.k * 2 + i * 2), Math.round(shot.y + shot.k - i), 1, 1); }
          if (shot.k > 22) shot = null;
        }
      }
      ctx.globalAlpha = 1;
    }
    build();
    let rt = 0;
    new ResizeObserver(() => { clearTimeout(rt); rt = setTimeout(build, 60); }).observe(cv);
    requestAnimationFrame(frame);
    return build;
  }
  window.ZB = { sky, starfield, seigaiha, landscape, PALETTES: P, FONTS, KEYS, DEFAULTS, ENGINES, load, save, apply, keymap, keyName, comboOf, toUrl, calc, sites, icon, mix };
})();
