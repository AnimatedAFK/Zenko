// Claude panel: plain claude.ai with night-shrine scrollbars so it blends with the browser.
(function () {
  if (window.top !== window) return;
  const css = `::-webkit-scrollbar{width:10px;height:10px}::-webkit-scrollbar-thumb{background:rgba(255,173,125,.22);border-radius:10px;border:3px solid transparent;background-clip:padding-box}::-webkit-scrollbar-thumb:hover{background:rgba(255,122,61,.45);background-clip:padding-box}::-webkit-scrollbar-track{background:transparent}`;
  function add() {
    if (document.getElementById("zb-panel-style")) return;
    const s = document.createElement("style");
    s.id = "zb-panel-style";
    s.textContent = css;
    (document.head || document.documentElement).appendChild(s);
  }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", add); else add();
})();
