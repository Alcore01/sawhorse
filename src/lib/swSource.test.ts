import { describe, expect, it } from "vitest";
import { buildVersion, serviceWorkerSource } from "./swSource";

describe("service worker source (review of 17 nov. 2026, constat 9)", () => {
  it("a new build is a new cache; the same files, the same one", () => {
    const a = ["index.html", "assets/index-AAAA.js", "assets/k.wasm"];
    expect(buildVersion(a)).toBe(buildVersion([...a].reverse()));
    expect(buildVersion(a)).not.toBe(buildVersion(["index.html", "assets/index-BBBB.js", "assets/k.wasm"]));
    const src = serviceWorkerSource(a);
    expect(src).toContain(`"volumecraft-${buildVersion(a)}"`);
    expect(src).toContain('"./assets/index-AAAA.js"');
  });

  it("drops older caches, keeps a page only when it came back whole, and caches nothing else", () => {
    const src = serviceWorkerSource(["index.html"]);
    expect(src).toMatch(/k\.startsWith\("volumecraft-"\) && k !== CACHE/);
    expect(src).toMatch(/if \(res\.ok\)/);
    expect(src).toMatch(/if \(!KEEP\.has\(url\.href\)\) return;/);
  });
});
