// sketchcraft E-58: Copy and Paste.
import { describe, expect, it } from "vitest";
import type { V3 } from "../kernel/types";
import { ev, fakeHost } from "./fakeHost";
import { PasteTool, anchorOf, pasteOps, type Clip } from "./paste";

const footprint: V3[] = [
  [2, 1, 0],
  [6, 1, 0],
  [6, 4, 0],
  [2, 4, 0],
];
const clip: Clip = {
  objects: [],
  nodes: [],
  faces: [{ rings: [footprint], normal: [0, 0, -1] }],
  segments: [],
  preview: footprint.map((p, k) => [p, footprint[(k + 1) % 4]] as [V3, V3]),
  anchor: anchorOf(footprint),
  what: "the outline of a face",
};

describe("Copy and Paste (sketchcraft E-58)", () => {
  it("the cursor holds the lowest corner", () => {
    expect(anchorOf([[3, 5, 2], [1, 7, 0], [4, 2, 6]])).toEqual([1, 2, 0]);
  });

  it("a building's base comes back as a flat face where it is put", () => {
    const ops = pasteOps(clip, [10, 0, 0]);
    expect(ops).toEqual([
      {
        op: "draw",
        target: { on: "plane", origin: [12, 1, 0], normal: [0, 0, -1] },
        points: [
          [12, 1, 0],
          [16, 1, 0],
          [16, 4, 0],
          [12, 4, 0],
        ],
        closed: true,
      },
    ]);
  });

  it("solids and groups are copied whole, moved by the same amount", () => {
    const ops = pasteOps({ ...clip, faces: [], objects: [3], nodes: [5] }, [1, 2, 0]);
    expect(ops[0]).toMatchObject({ op: "copy", objects: [3], nodes: [5], xform: { translation: [1, 2, 0] } });
  });

  it("the tool follows the cursor and puts it down on a click, as one step", async () => {
    const h = fakeHost();
    let done = 0;
    h.done = () => done++;
    const t = new PasteTool(h);
    t.setClip(clip);
    t.activate();
    t.onEvent(ev("move", [20, 30, 0]));
    const pts = t.view().overlay[0].points;
    expect(pts[0]).toEqual([20, 30, 0]); // the anchor corner under the cursor
    t.onEvent(ev("click", [20, 30, 0]));
    await Promise.resolve();
    await Promise.resolve();
    expect(h.ops.at(-1)).toMatchObject({ op: "draw", closed: true });
    expect(done).toBe(1);
  });

  it("Esc cancels: nothing pasted, back to Select", () => {
    const h = fakeHost();
    let done = 0;
    h.done = () => done++;
    const t = new PasteTool(h);
    t.setClip(clip);
    expect(t.cancel()).toBe(true);
    expect(h.ops).toHaveLength(0);
    expect(done).toBe(1);
  });
});

describe("Move with Ctrl on a face of a solid (sketchcraft E-58)", () => {
  it("copies the face's outline instead of reshaping the solid", async () => {
    const { moveOp } = await import("./move");
    const op = moveOp({ kind: "geometry", object: 1, points: footprint, what: "face 1", faceRings: [[footprint]] }, [0, 10, 0], true);
    expect(op).toMatchObject({ op: "draw", closed: true, points: [[2, 11, 0], [6, 11, 0], [6, 14, 0], [2, 14, 0]] });
    expect((op as { target: { normal: number[] } }).target.normal.map((x) => Math.abs(x))).toEqual([0, 0, 1]);
    // Without Ctrl the face itself moves, as before.
    expect(moveOp({ kind: "geometry", object: 1, points: footprint, what: "face 1", faceRings: [[footprint]] }, [0, 10, 0], false)).toMatchObject({ op: "transform_geometry" });
  });
});
