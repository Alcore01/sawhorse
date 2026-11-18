import { describe, expect, it } from "vitest";
import type { KernelOp, V3 } from "../kernel/types";
import { ev, fakeHost } from "./fakeHost";
import { DimensionTool, TapeTool, perpendicularFrom } from "./tape";
import type { Snap as ToolSnap } from "./tool";

const IN = 0.0254;
const tick = () => new Promise((r) => setTimeout(r, 0));
const last = (h: { ops: KernelOp[] }) => h.ops[h.ops.length - 1] as unknown as Record<string, unknown>;
const onEdge = (p: V3, a: V3, b: V3): ToolSnap => ({
  point: p,
  kind: "on-edge",
  tip: "On Edge",
  surface: null,
  entity: { kind: "edge", owner: { kind: "object", id: 1 }, a, b, drawn: false },
  lines: [],
});

describe("Tape measure", () => {
  it("perpendicular offsets from an edge", () => {
    expect(perpendicularFrom([0, 0, 0], [1, 0, 0], [0.4, 0, 0.3])).toEqual([0, 0, 0.3]);
  });

  it("from an edge: move away and type the offset to place a parallel guide", async () => {
    const h = fakeHost({ outcome: { kind: "guide_added", guide: 1 } });
    const t = new TapeTool(h);
    // The bottom edge of a side's inner face (x = 3/4 in, along y).
    const a: V3 = [0.75 * IN, 0, 0];
    const b: V3 = [0.75 * IN, 11.25 * IN, 0];
    t.onEvent(ev("click", [0.75 * IN, 5 * IN, 0], { snap: onEdge([0.75 * IN, 5 * IN, 0], a, b) }));
    expect(t.valueLabel()).toBe("Offset");
    t.onEvent(ev("move", [0.75 * IN, 5 * IN, 0.3], { snapKind: "on-face" }));
    t.onValue("21 1/2");
    await tick();
    const op = last(h) as { op: string; point: V3; dir: V3 };
    expect(op.op).toBe("add_guide");
    expect(op.dir).toEqual([0, 1, 0]);
    expect(op.point[0]).toBeCloseTo(0.75 * IN, 12);
    expect(op.point[2]).toBeCloseTo(21.5 * IN, 12);
  });

  it("point to point measures without drawing; a typed length drops a guide point", async () => {
    const h = fakeHost();
    const t = new TapeTool(h);
    t.onEvent(ev("click", [0, 0, 0], { snapKind: "endpoint" }));
    t.onEvent(ev("click", [0, 16 * IN, 0], { snapKind: "endpoint" }));
    expect(h.statuses.at(-1)?.text).toMatch(/Distance 16 in/);
    expect(h.ops).toHaveLength(0);
    t.onEvent(ev("click", [0, 0, 0], { snapKind: "endpoint" }));
    t.onEvent(ev("move", [1, 0, 0], { snapKind: "axis-x" }));
    t.onValue("36");
    await tick();
    expect(last(h)).toMatchObject({ op: "add_guide", dir: null });
    expect((last(h).point as V3)[0]).toBeCloseTo(36 * IN, 12);
  });
});

describe("Dimension", () => {
  it("two points, then the line's place: the offset is perpendicular to the span", async () => {
    const h = fakeHost();
    const t = new DimensionTool(h);
    t.onEvent(ev("click", [0, 0, 0], { snapKind: "endpoint" }));
    t.onEvent(ev("click", [1, 0, 0], { snapKind: "endpoint" }));
    t.onEvent(ev("move", [0.7, -0.2, 0], { snapKind: "free" }));
    expect(t.view().labels[0].text).toBe(h.fmt(1));
    t.onEvent(ev("click", [0.7, -0.2, 0], { snapKind: "free" }));
    await tick();
    expect(last(h)).toMatchObject({ op: "add_dimension", a: [0, 0, 0], b: [1, 0, 0] });
    const off = last(h).offset as V3;
    expect(off[0]).toBeCloseTo(0, 12);
    expect(off[1]).toBeCloseTo(-0.2, 12);
  });

  it("clicking an edge dimensions it", () => {
    const h = fakeHost();
    const t = new DimensionTool(h);
    t.onEvent(ev("click", [0.5, 0, 0], { snap: onEdge([0.5, 0, 0], [0, 0, 0], [2, 0, 0]) }));
    t.onEvent(ev("move", [1, 1, 0], { snapKind: "free" }));
    expect(t.view().labels[0].text).toBe(h.fmt(2));
  });

  it("snaps the line into the source face's plane or square to it (sketchcraft E-45)", async () => {
    // A roof pan rising towards +y: its normal leans back; the ridge-wise edge runs along x.
    const n: V3 = [0, -0.6, 0.8];
    const pan = { at: { kind: "face" as const, object: 1, face: 3 }, frame: { origin: [0, 0, 3] as V3, normal: n, u: [1, 0, 0] as V3, v: [0, 0.8, 0.6] as V3 }, point: [0, 0, 3] as V3, t: 9 };
    const onPan = (p: V3) => ({ snapKind: "endpoint" as const, picked: { entity: pan.at, point: p, surface: { ...pan, point: p } } });
    const inPlane = async (cursor: V3) => {
      const h = fakeHost();
      const t = new DimensionTool(h);
      t.onEvent(ev("click", [0, 0, 3], onPan([0, 0, 3])));
      t.onEvent(ev("click", [4, 0, 3], onPan([4, 0, 3])));
      t.onEvent(ev("move", cursor, { snapKind: "free" }));
      const status = h.statuses.at(-1)?.text ?? "";
      t.onEvent(ev("click", cursor, { snapKind: "free" }));
      await tick();
      return { off: last(h).offset as V3, status };
    };
    // Near the pan's own slope (0, 0.8, 0.6), a little off: it lies in the face.
    let r = await inPlane([2, 1.0, 3.9]);
    expect(r.status).toMatch(/in the face's plane/);
    expect(r.off[1] * 0.6 - r.off[2] * 0.8).toBeCloseTo(0, 9); // parallel to (0, 0.8, 0.6)
    // Near the normal, out of the roof: square to the face.
    r = await inPlane([2, -0.55, 3.85]);
    expect(r.status).toMatch(/square to the face/);
    expect(r.off[1] * 0.8 + r.off[2] * 0.6).toBeCloseTo(0, 9); // parallel to (0, -0.6, 0.8)
  });
});
