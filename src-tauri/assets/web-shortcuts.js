// Cephi shortcuts inside web pages. Pages have no IPC access, so a key press is reported as a
// navigation to cephi-key://<action>, which the app cancels and handles (see `on_navigation`).
(() => {
  if (window.top !== window) return;
  const report = (action) => {
    location.href = "cephi-key://" + action;
  };
  // Capture phase: runs before the page, but only for combinations Cephi owns.
  window.addEventListener(
    "keydown",
    (e) => {
      if (!e.ctrlKey || e.altKey || e.metaKey) return;
      let action;
      if (e.key === "Tab") action = e.shiftKey ? "tab-prev" : "tab-next";
      else if (e.shiftKey) return;
      // `code`: the digit row is the same on every keyboard layout.
      else if (/^Digit[1-4]$/.test(e.code)) action = "tab-" + e.code.slice(-1);
      else action = { n: "new-chat", e: "toggle-mode", l: "address" }[e.key.toLowerCase()];
      if (!action) return;
      e.preventDefault();
      e.stopImmediatePropagation();
      report(action);
    },
    true,
  );
  // Bubble phase: Escape only if the page didn't use it (closing a dialog, leaving fullscreen …).
  window.addEventListener("keydown", (e) => {
    if (e.key !== "Escape" || e.defaultPrevented || document.fullscreenElement) return;
    report("escape");
  });
})();
