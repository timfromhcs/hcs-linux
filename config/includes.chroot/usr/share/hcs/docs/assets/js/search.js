// Offline client-side search placeholder (Lunr.js vendored on-device).
const box = document.getElementById("q");
if (box) { box.addEventListener("input", (e) => { console.log("search:", e.target.value); }); }
