// sketchcraft E-34, review of 17 nov. 2026 (constat 9): the service worker is written by the
// build with the exact list of the build's files and a version drawn from that list. A new
// build is a new worker (the browser sees sw.js change), its files are fetched once, and the
// previous build's cache is dropped: storage does not grow update after update. Only those
// files are kept, cache first; the page is asked of the network first, kept only when it came
// back whole (a 404 never becomes the offline page); anything else (a render service on the
// same origin, its /healthz) always goes to the network.

/** A short, stable version for a list of files (FNV-1a over the sorted names). */
export function buildVersion(files: readonly string[]): string {
  let h = 0x811c9dc5;
  for (const f of [...files].sort()) {
    for (let i = 0; i < f.length; i++) {
      h ^= f.charCodeAt(i);
      h = Math.imul(h, 0x01000193) >>> 0;
    }
    h ^= 0x0a;
  }
  return h.toString(16).padStart(8, "0");
}

export function serviceWorkerSource(files: readonly string[]): string {
  const list = [...new Set(["./", ...files.map((f) => `./${f}`)])].sort();
  const cache = `volumecraft-${buildVersion(files)}`;
  return `// Written by the build (src/lib/swSource.ts): VolumeCraft offline, ${list.length} files.
const CACHE = ${JSON.stringify(cache)};
const FILES = ${JSON.stringify(list)};
const KEEP = new Set(FILES.map((f) => new URL(f, self.location).href));

self.addEventListener("install", (e) => {
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(FILES)).then(() => self.skipWaiting()));
});

self.addEventListener("activate", (e) => {
  e.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith("volumecraft-") && k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener("fetch", (e) => {
  const req = e.request;
  if (req.method !== "GET" || new URL(req.url).origin !== self.location.origin) return;
  if (req.mode === "navigate") {
    e.respondWith(
      fetch(req)
        .then((res) => {
          if (res.ok) {
            const copy = res.clone();
            caches.open(CACHE).then((c) => c.put("./", copy));
          }
          return res;
        })
        .catch(() => caches.match("./").then((r) => r || caches.match("./index.html"))),
    );
    return;
  }
  const url = new URL(req.url);
  url.search = "";
  if (!KEEP.has(url.href)) return;
  e.respondWith(caches.match(url.href).then((hit) => hit || fetch(req)));
});
`;
}
