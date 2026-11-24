// Runs in every website tab (top frame only): tells the browser the site's icon. Nothing else.
(function () {
  if (window.top !== window) return;
  function icon() {
    try {
      const links = Array.from(document.querySelectorAll('link[rel~="icon"], link[rel="shortcut icon"], link[rel="apple-touch-icon"]'));
      let best = links.find((l) => /32|48|any/.test(l.sizes && l.sizes.value || "")) || links[0];
      let href = best ? best.href : (location.protocol.startsWith("http") ? location.origin + "/favicon.ico" : "");
      if (href && window.ipc) window.ipc.postMessage(JSON.stringify({ t: "icon", href }));
    } catch (e) {}
  }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", icon);
  else icon();
  window.addEventListener("load", () => setTimeout(icon, 300));
})();
