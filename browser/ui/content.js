// Runs in every website tab (top frame only). It reports the site's icon, and when the browser answers
// "hello" it switches on the helpers you chose: per-site dark mode / hiding bits, cookie pop-up rejecting,
// YouTube ad and sponsor skipping, mouse gestures, link preloading, draft keeping, scroll restore,
// video tools, keyboard link hints, citations and the InThinking -> Zenko notes button.
(function () {
  if (window.top !== window || window.__zb) return;
  const post = (o) => { try { window.ipc && window.ipc.postMessage(JSON.stringify(o)); } catch (e) {} };
  const host = location.hostname.replace(/^www\./, "");
  let cfg = {};
  let touched = false;
  addEventListener("pointerdown", () => { touched = true; }, true);
  addEventListener("keydown", () => { touched = true; }, true);

  function icon() {
    try {
      const links = Array.from(document.querySelectorAll('link[rel~="icon"], link[rel="shortcut icon"], link[rel="apple-touch-icon"]'));
      const best = links.find((l) => /32|48|any/.test((l.sizes && l.sizes.value) || "")) || links[0];
      const href = best ? best.href : (location.protocol.startsWith("http") ? location.origin + "/favicon.ico" : "");
      if (href) post({ t: "icon", href });
    } catch (e) {}
  }
  function css(id, text) {
    let s = document.getElementById(id);
    if (!text) { if (s) s.remove(); return; }
    if (!s) { s = document.createElement("style"); s.id = id; (document.head || document.documentElement).appendChild(s); }
    s.textContent = text;
  }

  // ---------------------------------------------------------------- per-site look
  const DARK = `html{filter:invert(.9) hue-rotate(180deg)!important;background:#fff!important}
    img,video,picture,canvas,iframe,svg image,[style*="background-image"]{filter:invert(1) hue-rotate(180deg)!important}`;
  const SHORTS = `ytd-reel-shelf-renderer,ytd-rich-shelf-renderer[is-shorts],ytd-guide-entry-renderer a[title="Shorts"],
    ytd-mini-guide-entry-renderer[aria-label="Shorts"],a[href^="/shorts"],ytd-video-renderer:has(a[href^="/shorts"]),
    ytd-grid-video-renderer:has(a[href^="/shorts"]),ytd-rich-item-renderer:has(a[href^="/shorts"]),
    yt-tab-shape[tab-title="Shorts"],grid-shelf-view-model:has(a[href^="/shorts"]){display:none!important}`;
  const READ = `body,p,li,td,input,textarea{font-family:"Segoe UI Variable Text","Segoe UI",system-ui,sans-serif!important;line-height:1.6!important}`;
  function look() {
    const s = cfg.site || {};
    css("__zb_dark", s.dark ? DARK : "");
    css("__zb_shorts", s.hideShorts && /youtube\.com$/.test(host) ? SHORTS : "");
    css("__zb_read", s.font === "readable" ? READ : "");
    css("__zb_hide", s.hide ? String(s.hide).split(/\n|,/).map((x) => x.trim()).filter(Boolean).join(",") + "{display:none!important}" : "");
  }

  // ---------------------------------------------------------------- cookie pop-ups: say "reject" for you
  const REJECT = ["#onetrust-reject-all-handler", "#CybotCookiebotDialogBodyButtonDecline", "button.fc-cta-do-not-consent",
    ".qc-cmp2-summary-buttons button[mode=secondary]", "button[aria-label='Reject all']", "button[aria-label='Reject All']",
    "[data-testid='uc-deny-all-button']", "#didomi-notice-disagree-button", ".cky-btn-reject", "#truste-consent-required",
    "button.sp_choice_type_REJECT_ALL", ".osano-cm-denyAll", "#tarteaucitronAllDenied2", ".cmplz-deny", "#cookie_action_close_header_reject"];
  const WORDS = /^(reject all|reject|decline|decline all|deny|deny all|only necessary|necessary only|use necessary cookies only|refuser|tout refuser|continuer sans accepter|nur notwendige|alle ablehnen)$/i;
  function cookies() {
    let n = 0;
    const tick = () => {
      if (++n > 20) return;
      for (const sel of REJECT) { const b = document.querySelector(sel); if (b && b.offsetParent) { b.click(); return; } }
      const bs = document.querySelectorAll("button,[role=button],a.button");
      for (const b of bs) {
        const t = (b.innerText || b.textContent || "").trim();
        if (t.length < 40 && WORDS.test(t) && b.offsetParent && /cookie|consent|privacy|gdpr|cmp|banner/i.test((b.closest("[id],[class]") || {}).outerHTML?.slice(0, 400) || "")) { b.click(); return; }
      }
      setTimeout(tick, 700);
    };
    setTimeout(tick, 600);
  }

  // ---------------------------------------------------------------- YouTube: skip ads + sponsor segments
  function ytAds() {
    setInterval(() => {
      const p = document.querySelector(".html5-video-player");
      if (!p) return;
      const v = p.querySelector("video");
      if (p.classList.contains("ad-showing") && v) {
        v.muted = true;
        if (isFinite(v.duration)) v.currentTime = Math.max(v.currentTime, v.duration - 0.1);
        v.playbackRate = 16;
      }
      const skip = document.querySelector(".ytp-ad-skip-button, .ytp-ad-skip-button-modern, .ytp-skip-ad-button, button[id^='skip-button']");
      if (skip) skip.click();
      document.querySelectorAll("ytd-ad-slot-renderer, ytd-promoted-sparkles-web-renderer, #player-ads, ytd-in-feed-ad-layout-renderer").forEach((e) => { e.style.display = "none"; });
    }, 400);
  }
  let segs = [], segFor = "";
  function sponsor() {
    setInterval(async () => {
      const id = new URLSearchParams(location.search).get("v");
      if (!id || id === segFor) return;
      segFor = id; segs = [];
      try {
        const r = await fetch("https://sponsor.ajay.app/api/skipSegments?videoID=" + id + "&categories=" + encodeURIComponent('["sponsor","selfpromo","interaction"]'));
        if (r.ok) segs = (await r.json()).map((s) => s.segment);
      } catch (e) {}
    }, 1500);
    document.addEventListener("timeupdate", (e) => {
      const v = e.target;
      if (!(v instanceof HTMLVideoElement) || !segs.length) return;
      for (const [a, b] of segs) if (v.currentTime >= a && v.currentTime < b - 0.3) { v.currentTime = b; break; }
    }, true);
  }

  // ---------------------------------------------------------------- no autoplay (until you click or type here)
  function noAutoplay() {
    document.addEventListener("play", (e) => {
      if (!touched && e.target instanceof HTMLMediaElement) { try { e.target.pause(); } catch (x) {} }
    }, true);
  }

  // ---------------------------------------------------------------- mouse gestures (hold the right button and draw)
  function gestures() {
    let x0 = 0, y0 = 0, path = "", on = false, used = false;
    addEventListener("mousedown", (e) => { if (e.button === 2) { on = true; used = false; x0 = e.clientX; y0 = e.clientY; path = ""; } }, true);
    addEventListener("mousemove", (e) => {
      if (!on) return;
      const dx = e.clientX - x0, dy = e.clientY - y0;
      if (Math.abs(dx) < 28 && Math.abs(dy) < 28) return;
      const d = Math.abs(dx) > Math.abs(dy) ? (dx > 0 ? "R" : "L") : (dy > 0 ? "D" : "U");
      if (path[path.length - 1] !== d) path += d;
      x0 = e.clientX; y0 = e.clientY;
    }, true);
    addEventListener("mouseup", (e) => {
      if (e.button !== 2 || !on) return;
      on = false;
      const g = { L: "back", R: "fwd", UD: "reload", DR: "close", U: "new", D: "reopen" }[path];
      if (g) { used = true; post({ t: "gesture", g }); }
    }, true);
    addEventListener("contextmenu", (e) => { if (used) { e.preventDefault(); e.stopPropagation(); used = false; } }, true);
  }

  // ---------------------------------------------------------------- preload links you hover (same site only)
  function preload() {
    const done = new Set();
    let t = 0;
    addEventListener("mouseover", (e) => {
      const a = e.target.closest && e.target.closest("a[href]");
      if (!a) return;
      clearTimeout(t);
      t = setTimeout(() => {
        try {
          const u = new URL(a.href);
          if (u.origin !== location.origin || done.has(u.href) || done.size > 30 || u.href === location.href) return;
          done.add(u.href);
          const l = document.createElement("link"); l.rel = "prefetch"; l.href = u.href; document.head.appendChild(l);
        } catch (x) {}
      }, 120);
    }, { passive: true });
  }

  // ---------------------------------------------------------------- keep what you type (survives reloads and crashes)
  function drafts() {
    const key = (el) => "__zbd:" + location.pathname + ":" + (el.name || el.id || "");
    const ok = (el) => (el instanceof HTMLTextAreaElement || (el instanceof HTMLInputElement && /^(text|search|)$/.test(el.type))) && (el.name || el.id) && !/pass|card|cvv|otp|code/i.test(el.name + el.id);
    addEventListener("input", (e) => { const el = e.target; if (ok(el)) try { sessionStorage.setItem(key(el), el.value.slice(0, 20000)); } catch (x) {} }, true);
    addEventListener("submit", () => { try { Object.keys(sessionStorage).filter((k) => k.startsWith("__zbd:" + location.pathname)).forEach((k) => sessionStorage.removeItem(k)); } catch (x) {} }, true);
    setTimeout(() => {
      document.querySelectorAll("textarea, input[type=text], input:not([type])").forEach((el) => {
        if (!ok(el) || el.value) return;
        try { const v = sessionStorage.getItem(key(el)); if (v) { el.value = v; el.dispatchEvent(new Event("input", { bubbles: true })); } } catch (x) {}
      });
    }, 900);
  }

  // ---------------------------------------------------------------- InThinking: key terms -> Zenko's notes
  function inthinking() {
    if (document.getElementById("__zb_it")) return;
    const b = document.createElement("button");
    b.id = "__zb_it";
    b.textContent = "🦊 Send key terms to Zenko";
    b.style.cssText = "position:fixed;right:18px;bottom:18px;z-index:2147483647;padding:9px 14px;border:2px solid #e8b54a;background:#141b3a;color:#f3ead2;font:600 14px 'Segoe UI',sans-serif;cursor:pointer;border-radius:4px;box-shadow:0 4px 18px #0006";
    b.onclick = () => {
      const lines = [];
      const title = (document.querySelector("h1") || {}).innerText || document.title;
      lines.push("# " + title.trim());
      document.querySelectorAll("main h2, main h3, article h2, article h3, .content h2, .content h3, h2, h3").forEach((h) => {
        const t = h.innerText.trim(); if (t && t.length < 120 && !lines.includes("## " + t)) lines.push("## " + t);
      });
      const seen = new Set();
      document.querySelectorAll("strong, b, dt").forEach((s) => {
        const term = s.innerText.trim().replace(/[:.]$/, "");
        if (term.length < 3 || term.length > 60 || seen.has(term.toLowerCase())) return;
        seen.add(term.toLowerCase());
        const p = s.closest("p, li, dd, td") || s.parentElement;
        let def = (p ? p.innerText : "").replace(/\s+/g, " ").trim();
        if (def.toLowerCase().startsWith(term.toLowerCase())) def = def.slice(term.length).replace(/^[\s:–—-]+/, "");
        lines.push(term + " — " + def.slice(0, 240));
      });
      post({ t: "notes", s: lines.slice(0, 80).join("\n") + "\n(from " + location.href + ")" });
      b.textContent = "✓ Sent to Zenko";
      setTimeout(() => { b.textContent = "🦊 Send key terms to Zenko"; }, 2500);
    };
    document.body.appendChild(b);
  }

  // ---------------------------------------------------------------- Outlook: tell the browser about new mail
  function outlook() {
    let last = -1, t = 0;
    const count = () => {
      // the unread number next to "Inbox" in the folder list
      const el = Array.from(document.querySelectorAll('[role="treeitem"], [role="option"]')).find((e) => /^inbox/i.test((e.getAttribute("aria-label") || e.innerText || "").trim()));
      const m = el && (el.getAttribute("aria-label") || el.innerText || "").match(/(\d+)\s*(unread|non lu)/i) || (el && (el.innerText || "").match(/\n\s*(\d+)\s*$/));
      return m ? parseInt(m[1], 10) : 0;
    };
    new MutationObserver(() => {
      clearTimeout(t);
      t = setTimeout(() => {
        const n = count();
        if (last >= 0 && n > last) post({ t: "mail" });
        last = n;
      }, 800);
    }).observe(document.documentElement, { subtree: true, childList: true, characterData: true });
  }

  // ---------------------------------------------------------------- video tools
  function vid() {
    const vs = Array.from(document.querySelectorAll("video"));
    return vs.find((v) => !v.paused) || vs.sort((a, b) => b.clientWidth * b.clientHeight - a.clientWidth * a.clientHeight)[0];
  }
  function toast(msg) {
    let d = document.getElementById("__zb_toast");
    if (!d) {
      d = document.createElement("div"); d.id = "__zb_toast";
      d.style.cssText = "position:fixed;left:50%;top:18px;transform:translateX(-50%);z-index:2147483647;padding:8px 14px;background:#141b3acc;color:#f3ead2;border:2px solid #e8b54a;font:600 15px 'Segoe UI',sans-serif;border-radius:4px;pointer-events:none;transition:opacity .25s";
      document.documentElement.appendChild(d);
    }
    d.textContent = msg; d.style.opacity = "1";
    clearTimeout(d._t); d._t = setTimeout(() => { d.style.opacity = "0"; }, 1200);
  }
  function video(a, val) {
    const v = vid();
    if (!v) { toast("No video on this page"); return; }
    if (a === "speed") { v.playbackRate = Math.min(4, Math.max(0.25, Math.round((v.playbackRate + (val || 0.25)) * 100) / 100)); toast("Speed " + v.playbackRate + "×"); }
    else if (a === "speedSet") { v.playbackRate = val || 1; toast("Speed " + v.playbackRate + "×"); }
    else if (a === "loop") { v.loop = !v.loop; toast(v.loop ? "Loop on" : "Loop off"); }
    else if (a === "pip") { (document.pictureInPictureElement ? document.exitPictureInPicture() : v.requestPictureInPicture()).catch(() => toast("This video can't float")); }
    else if (a === "play") { v.paused ? v.play() : v.pause(); }
    else if (a === "skip") { v.currentTime += val || 10; }
  }

  // ---------------------------------------------------------------- keyboard link hints
  function hints() {
    document.querySelectorAll(".__zb_h").forEach((e) => e.remove());
    const els = Array.from(document.querySelectorAll("a[href], button, input:not([type=hidden]), select, textarea, [role=button], [role=link], [onclick], summary"))
      .filter((e) => { const r = e.getBoundingClientRect(); return r.width > 2 && r.height > 2 && r.bottom > 0 && r.right > 0 && r.top < innerHeight && r.left < innerWidth && getComputedStyle(e).visibility !== "hidden"; })
      .slice(0, 400);
    if (!els.length) return;
    const A = "asdfghjklqweruiopzxcvnm";
    const two = els.length > A.length;
    const labels = els.map((_, i) => two ? A[Math.floor(i / A.length) % A.length] + A[i % A.length] : A[i]);
    const box = document.createElement("div");
    box.className = "__zb_h";
    box.style.cssText = "position:fixed;inset:0;z-index:2147483647;pointer-events:none";
    els.forEach((e, i) => {
      const r = e.getBoundingClientRect();
      const s = document.createElement("span");
      s.textContent = labels[i].toUpperCase();
      s.dataset.l = labels[i];
      s.style.cssText = `position:fixed;left:${Math.max(0, r.left - 4)}px;top:${Math.max(0, r.top - 4)}px;padding:1px 4px;background:#e8b54a;color:#141b3a;font:700 12px/1.25 Consolas,monospace;border:1px solid #141b3a;border-radius:3px;box-shadow:0 2px 6px #0005`;
      box.appendChild(s);
    });
    document.documentElement.appendChild(box);
    let typed = "";
    const done = () => { box.remove(); removeEventListener("keydown", key, true); };
    const key = (e) => {
      e.preventDefault(); e.stopPropagation();
      if (e.key === "Escape") return done();
      if (e.key === "Backspace") typed = typed.slice(0, -1); else if (e.key.length === 1) typed += e.key.toLowerCase();
      const i = labels.indexOf(typed);
      box.querySelectorAll("span").forEach((s) => { s.style.opacity = s.dataset.l.startsWith(typed) ? "1" : ".15"; });
      if (i >= 0) {
        done();
        const el = els[i];
        if (/^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName)) el.focus(); else el.click();
      } else if (!labels.some((l) => l.startsWith(typed))) done();
    };
    addEventListener("keydown", key, true);
  }

  // ---------------------------------------------------------------- citation details
  function cite() {
    const m = (sel) => { const e = document.querySelector(sel); return e ? (e.getAttribute("content") || e.textContent || "").trim() : ""; };
    let author = m('meta[name="author"]') || m('meta[property="article:author"]') || m('meta[name="citation_author"]') || m('[rel="author"]') || m('[itemprop="author"] [itemprop="name"]') || m('[itemprop="author"]');
    if (/^https?:/.test(author)) author = "";
    const date = m('meta[property="article:published_time"]') || m('meta[name="citation_publication_date"]') || m('meta[name="date"]') || m('meta[itemprop="datePublished"]') || (document.querySelector("time[datetime]") || { getAttribute: () => "" }).getAttribute("datetime");
    const site = m('meta[property="og:site_name"]') || host;
    const title = m('meta[property="og:title"]') || m('meta[name="citation_title"]') || document.title;
    post({ t: "cite", title, author, date, site, url: location.href });
  }

  window.__zb = {
    tweak(c) {
      const first = !cfg.__done;
      cfg = c || {}; cfg.__done = true;
      look();
      if (!first) return;
      if (cfg.cookies) cookies();
      if (/youtube\.com$/.test(host)) { if (cfg.ytAds) ytAds(); if (cfg.sponsor) sponsor(); }
      if (cfg.noAutoplay) noAutoplay();
      if (cfg.gestures) gestures();
      if (cfg.preload) preload();
      if (cfg.drafts) drafts();
      if (cfg.inthinking) inthinking();
      if (cfg.outlook) outlook();
      if (cfg.scroll > 0) { const go = () => scrollTo(0, cfg.scroll); go(); setTimeout(go, 600); setTimeout(go, 1600); }
    },
    video, hints, cite,
  };

  // scroll position + page text (for "search inside open tabs")
  let st = 0;
  addEventListener("scroll", () => { clearTimeout(st); st = setTimeout(() => post({ t: "scroll", y: Math.round(scrollY) }), 1500); }, { passive: true });
  function text() { try { post({ t: "text", s: (document.title + "\n" + (document.body ? document.body.innerText : "")).slice(0, 30000) }); } catch (e) {} }

  function hello() { post({ t: "hello" }); icon(); }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", hello); else hello();
  addEventListener("load", () => { setTimeout(icon, 300); setTimeout(text, 2500); });
})();
