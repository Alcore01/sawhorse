// sketchcraft E-31: Push along an axis, the other way to push (JC, 17 nov. 2026, 09 h 01).
import { describe, expect, it } from "vitest";
import type { KernelOp, Picked, SurfaceHit } from "../kernel/types";
import { ev, fakeHost } from "./fakeHost";
import { PushAxisTool } from "./pushaxis";

const slope = (region: number): Picked => {
  const surface: SurfaceHit = {
    at: { kind: "region", sketch: 3, region },
    frame: { origin: [1, 0.25, 1.25], normal: [0, -0.7071, 0.7071], u: [1, 0, 0], v: [0, 0.7071, 0.7071] },
    point: [1, 0.25, 1.25],
    t: 9,
  };
  return { entity: { kind: "region", sketch: 3, region }, point: surface.point, surface };
};
const tick = () => new Promise((r) => setTimeout(r, 0));
const last = (h: { ops: KernelOp[] }) => h.ops[h.ops.length - 1] as unknown as Record<string, unknown>;

describe("Push along an axis (E-31)", () => {
  it("pushes every selected face together straight up blue, not along the normal", async () => {
    const h = fakeHost();
    h.sel = [{ kind: "region", sketch: 3, region: 1 }, { kind: "region", sketch: 4, region: 2 }];
    const t = new PushAxisTool(h);
    t.onEvent(ev("down", [1, 0.25, 1.25], { picked: slope(1) }));
    await tick();
    t.onEvent({ ...ev("move", [1, 0.25, 1.55], { picked: slope(1), snapKind: "axis-z" }), type: "drag", start: [0, 0] });
    expect(t.view().overlay.length).toBeGreaterThan(0);
    t.onEvent({ ...ev("move", [1, 0.25, 1.55], { picked: slope(1) }), type: "dragend", start: [0, 0] });
    await tick();
    expect(last(h)).toMatchObject({ op: "extrude_faces", faces: [[3, 1], [4, 2]] });
    const v = last(h).vector as number[];
    expect(v[0]).toBeCloseTo(0, 12);
    expect(v[1]).toBeCloseTo(0, 12);
    expect(v[2]).toBeCloseTo(0.3, 12);
  });

  it("a face not in the selection goes alone; the arrows switch the axis; a typed value commits", async () => {
    const h = fakeHost();
    h.sel = [{ kind: "region", sketch: 4, region: 2 }];
    const t = new PushAxisTool(h);
    t.onEvent(ev("down", [1, 0.25, 1.25], { picked: slope(1) }));
    t.onEvent(ev("click", [1, 0.25, 1.25], { picked: slope(1) }));
    expect(t.inferContext().lock?.kind).toBe("axis-z");
    t.onKey(new KeyboardEvent("keydown", { key: "ArrowLeft" }));
    expect(t.inferContext().lock?.kind).toBe("axis-y");
    t.onValue("50cm");
    await tick();
    expect(last(h)).toMatchObject({ op: "extrude_faces", faces: [[3, 1]] });
    expect((last(h).vector as number[])[1]).toBeCloseTo(0.5, 12);
  });

  it("a solid's face is sent to Push/Pull", () => {
    const h = fakeHost();
    const t = new PushAxisTool(h);
    const face: SurfaceHit = { at: { kind: "face", object: 1, face: 2 }, frame: { origin: [0, 0, 1], normal: [0, 0, 1], u: [1, 0, 0], v: [0, 1, 0] }, point: [0, 0, 1], t: 9 };
    t.onEvent(ev("down", [0, 0, 1], { picked: { entity: face.at, point: face.point, surface: face } }));
    expect(t.busy()).toBe(false);
    expect(h.statuses.at(-1)?.text ?? "").toMatch(/Push\/Pull/);
  });
});
