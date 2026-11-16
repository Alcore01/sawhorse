import { describe, expect, it } from "vitest";
import type { KernelOp, V3 } from "../kernel/types";
import { TOOLS } from "../lib/tools";
import { TOOL_GLYPHS } from "../components/glyphs";
import { ev, fakeHost } from "./fakeHost";
import { ProtractorTool, fmtSlope, parseProtractorAngle, referenceAngle } from "./protractor";

const DEG = Math.PI / 180;
const tick = () => new Promise((r) => setTimeout(r, 0));
const lastOp = (h: { ops: KernelOp[] }) => h.ops[h.ops.length - 1] as unknown as { op: string; point: V3; dir: V3 | null };
const key = (k: string) => ({ key: k }) as KeyboardEvent;
const close = (a: V3, b: V3) => a.forEach((x, i) => expect(x).toBeCloseTo(b[i], 12));

describe("Protractor: typed angles", () => {
  it("reads degrees, a slope in percent and a rise and run", () => {
    expect(parseProtractorAngle("35")).toBe(35);
    expect(parseProtractorAngle("-22.5°")).toBe(-22.5);
    expect(parseProtractorAngle("100%")).toBeCloseTo(45, 12);
    expect(parseProtractorAngle("70 %")).toBeCloseTo(Math.atan(0.7) / DEG, 12);
    expect(parseProtractorAngle("1:2")).toBeCloseTo(Math.atan(0.5) / DEG, 12);
    expect(parseProtractorAngle("1:0")).toBeNull();
    expect(parseProtractorAngle("12 in")).toBeNull();
    expect(parseProtractorAngle("")).toBeNull();
  });

  it("shows a pitch with its slope", () => {
    expect(fmtSlope(45 * DEG)).toBe("45° (100%)");
    expect(fmtSlope(90 * DEG)).toBe("90°");
  });
});

describe("Protractor tool", () => {
  it("on the ground: vertex, reference, then a click at 45 degrees places a guide through the vertex", async () => {
    const h = fakeHost({ outcome: { kind: "guide_added", guide: 1 } });
    const t = new ProtractorTool(h);
    t.onEvent(ev("click", [0, 0, 0], { snapKind: "endpoint" }));
    t.onEvent(ev("click", [1, 0, 0], { snapKind: "axis-x" }));
    t.onEvent(ev("move", [1, 1.05, 0], { snapKind: "free" }));
    expect(h.values.at(-1)).toBe("45°"); // snapped to the tick
    t.onEvent(ev("click", [1, 1.05, 0], { snapKind: "free" }));
    await tick();
    const op = lastOp(h);
    expect(op.op).toBe("add_guide");
    close(op.point, [0, 0, 0]);
    close(op.dir as V3, [Math.SQRT1_2, Math.SQRT1_2, 0]);
    expect(h.statuses.at(-1)?.text).toMatch(/Guide at 45° \(100%\)/);
    expect(t.busy()).toBe(false);
  });

  it("a typed slope goes on the side of the reference the cursor is on", async () => {
    const h = fakeHost({ outcome: { kind: "guide_added", guide: 1 } });
    const t = new ProtractorTool(h);
    t.onEvent(ev("click", [0, 0, 0], { snapKind: "endpoint" }));
    t.onEvent(ev("click", [1, 0, 0], { snapKind: "axis-x" }));
    t.onEvent(ev("move", [1, -0.3, 0], { snapKind: "free" }));
    t.onValue("70%");
    await tick();
    const a = Math.atan(0.7);
    close(lastOp(h).dir as V3, [Math.cos(a), -Math.sin(a), 0]);
  });

  it("locked upright facing green, a 1:2 roof pitch rises from the eaves line", async () => {
    const h = fakeHost({ outcome: { kind: "guide_added", guide: 1 } });
    const t = new ProtractorTool(h);
    expect(t.onKey(key("ArrowLeft"))).toBe(true);
    t.onEvent(ev("move", [2, 0, 3], { snapKind: "endpoint" }));
    t.onEvent(ev("click", [2, 0, 3], { snapKind: "endpoint" })); // the eaves corner
    t.onEvent(ev("click", [5, 0, 3], { snapKind: "on-edge" })); // along the eaves
    t.onEvent(ev("move", [4, 0, 4], { snapKind: "free" })); // up
    t.onValue("1:2");
    await tick();
    const op = lastOp(h);
    close(op.point, [2, 0, 3]);
    const a = Math.atan(0.5);
    close(op.dir as V3, [Math.cos(a), 0, Math.sin(a)]);
  });

  it("an angle typed before the vertex is refused in words, and nothing is placed", () => {
    const h = fakeHost();
    const t = new ProtractorTool(h);
    t.onValue("35");
    expect(h.statuses.at(-1)).toMatchObject({ error: true });
    t.onEvent(ev("click", [0, 0, 0], { snapKind: "endpoint" }));
    t.onValue("north");
    expect(h.statuses.at(-1)?.text).toMatch(/degrees/);
    expect(h.ops).toHaveLength(0);
  });

  it("is on the tool rail, with its own glyph and the angle chip", () => {
    const def = TOOLS.find((d) => d.id === "protractor");
    expect(def?.vcb).toBe("Angle");
    expect(TOOL_GLYPHS.protractor).toBeTruthy();
  });
});

describe("Protractor: what the angle is measured from (sketchcraft E-20)", () => {
  it("a reference line's angle is read from the plane's level (0 to 90 degrees), either way along it", () => {
    const a = 37 * DEG;
    const n: V3 = [0, 1, 0];
    expect(referenceAngle([Math.cos(a), 0, Math.sin(a)], n) / DEG).toBeCloseTo(37, 9);
    expect(referenceAngle([-Math.cos(a), 0, -Math.sin(a)], n) / DEG).toBeCloseTo(37, 9);
    expect(referenceAngle([Math.cos(a), 0, -Math.sin(a)], n) / DEG).toBeCloseTo(37, 9);
    expect(referenceAngle([1, 0, 0], n) / DEG).toBeCloseTo(0, 9);
    // On a level plane, from red toward green.
    expect(referenceAngle([0, 1, 0], [0, 0, 1]) / DEG).toBeCloseTo(90, 9);
  });

  it("before the reference is clicked the protractor turns with the cursor and says the reference's angle; then its zero is the reference", () => {
    const h = fakeHost();
    const t = new ProtractorTool(h);
    t.onKey(key("ArrowLeft"));
    t.onEvent(ev("click", [0, 0, 0], { snapKind: "endpoint" }));
    const a = 37 * DEG;
    // The underside of a roof at 37 degrees, upright facing green.
    t.onEvent(ev("move", [Math.cos(a), 0, Math.sin(a)], { snapKind: "on-edge" }));
    const v = t.view();
    expect(v.labels.map((l) => l.text)).toContain("ref 37° (75.4%)");
    // The base line of the half protractor runs along the cursor's direction.
    const base = v.overlay.find((o) => o.kind === "segments" && o.points.length === 2 && o.width === 1.5) as { points: V3[] };
    const d = [base.points[1][0] - base.points[0][0], base.points[1][2] - base.points[0][2]];
    expect(Math.atan2(d[1], d[0]) / DEG).toBeCloseTo(37, 6);
    t.onEvent(ev("click", [Math.cos(a), 0, Math.sin(a)], { snapKind: "on-edge" }));
    expect(t.view().labels.map((l) => l.text)).toContain("0°");
  });
});
