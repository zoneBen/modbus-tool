/* Applies the stored theme before first paint. Kept CSP-safe (external, 'self'). */
(function () {
  try {
    var raw = localStorage.getItem("mt.theme");
    var preference = raw ? raw.replace(/^"|"$/g, "") : "dark";
    var dark =
      preference === "system"
        ? window.matchMedia("(prefers-color-scheme: dark)").matches
        : preference !== "light";
    var root = document.documentElement;
    root.classList.toggle("dark", dark);
    root.style.colorScheme = dark ? "dark" : "light";
    root.dataset.theme = dark ? "dark" : "light";
  } catch {
    document.documentElement.classList.add("dark");
  }
})();
