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
    palette: "classic", accent: "", font: "segoe", tabShape: "ema", tabs: "top", radius: 12, density: "normal",
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
    const c = P[p.palette] || P.classic;
    const acc = /^#[0-9a-f]{6}$/i.test(p.accent || "") ? p.accent : c[5];
    const v = {
      "--night": c[0], "--paper": c[1], "--card": c[2], "--ink": c[3], "--grey": c[4], "--acc": acc,
      "--accl": p.accent ? mix(acc, "#ffffff", 0.35) : c[6], "--gold": c[7], "--glow": c[8],
      "--night2": mix(c[0], acc, 0.14), "--field": mix(c[2], c[3], 0.07), "--line": mix(c[2], c[3], 0.14),
      "--edge": mix(c[2], c[3], 0.28), "--hover": mix(c[2], c[3], 0.10), "--accbg": mix(c[2], acc, 0.22),
      "--accdim": mix(c[1], acc, 0.35), "--font": (FONTS[p.font] || FONTS.segoe)[0], "--r": (p.radius | 0) + "px",
      "--rs": Math.max(2, Math.round((p.radius | 0) * 0.6)) + "px",
    };
    for (const k in v) root.style.setProperty(k, v[k]);
    root.dataset.shape = p.tabShape;
    root.dataset.tabs = p.tabs;
    root.dataset.density = p.density;
    root.dataset.motion = p.motion;
    root.dataset.glow = p.glow ? "on" : "off";
    root.dataset.stars = p.stars ? "on" : "off";
    root.style.fontSize = (p.uiScale || 100) * 0.13 + "px";
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
    star: ["...#...", "..###..", "#######", ".#####.", "..###..", ".##.##.", "#.....#"],
  };
  function icon(name, px) {
    const rows = ICONS[name] || ICONS.star;
    const h = rows.length, w = Math.max(...rows.map((r) => r.length));
    let r = "";
    rows.forEach((row, y) => { for (let x = 0; x < row.length; x++) if (row[x] === "#") r += `<rect x="${x}" y="${y}" width="1.02" height="1.02"/>`; });
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
  window.ZB = { starfield, PALETTES: P, FONTS, KEYS, DEFAULTS, ENGINES, load, save, apply, keymap, keyName, comboOf, toUrl, calc, sites, icon, mix };
})();
