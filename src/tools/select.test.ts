import { describe, expect, it } from "vitest";
import type { Entity, Picked } from "../kernel/types";
import { EraseTool } from "./erase";
import { ev, fakeHost } from "./fakeHost";
import { SelectTool } from "./select";

const face: Entity = { kind: "face", object: 1, face: 2 };
const other: Entity = { kind: "face", object: 1, face: 3 };
const edge: Entity = { kind: "edge", owner: { kind: "object", id: 1 }, a: [0, 0, 1], b: [1, 0, 1], drawn: true };
const pickOf = (e: Entity): Picked => ({ entity: e, point: [0.5, 0.5, 1], surface: null });

describe("Select tool", () => {
  it("hovers, clicks, adds with Shift, toggles with Ctrl, clears on empty click", () => {
    const h = fakeHost();
    const t = new SelectTool(h);
    t.onEvent(ev("move", [0.5, 0.5, 1], { picked: pickOf(face) }));
    expect(h.hovered).toEqual(face);
    t.onEvent(ev("click", [0.5, 0.5, 1], { picked: pickOf(face) }));
    expect(h.sel).toEqual([face]);
    t.onEvent(ev("click", [0.5, 0.5, 1], { picked: pickOf(other), shift: true }));
    expect(h.sel).toEqual([face, other]);
    t.onEvent(ev("click", [0.5, 0.5, 1], { picked: pickOf(face), ctrl: true }));
    expect(h.sel).toEqual([other]);
    t.onEvent(ev("click", [0.5, 0.5, 1], { picked: pickOf(edge), ctrl: true }));
    expect(h.sel).toEqual([other, edge]);
    t.onEvent(ev("click", [5, 5, 0], { picked: null }));
    expect(h.sel).toEqual([]);
  });

  it("double-click selects the whole solid", () => {
    const h = fakeHost({ _state: { groups: [] } as never });
    const t = new SelectTool(h);
    t.onEvent(ev("dblclick", [0.5, 0.5, 1], { picked: pickOf(face) }));
    expect(h.sel).toEqual([{ kind: "object", object: 1 }]);
  });

  it("double-click on a loose face selects every connected face (sketchcraft E-37)", async () => {
    const asked: [number, number][] = [];
    const h = fakeHost({
      _state: { groups: [] } as never,
      connectedFaces: async (sketch: number, region: number) => {
        asked.push([sketch, region]);
        return [[2, 1], [3, 4], [5, 2]];
      },
    });
    const t = new SelectTool(h);
    const wall: Entity = { kind: "region", sketch: 3, region: 4 };
    t.onEvent(ev("dblclick", [0.5, 0.5, 1], { picked: pickOf(wall) }));
    await new Promise((r) => setTimeout(r, 0));
    expect(asked).toEqual([[3, 4]]);
    expect(h.sel).toEqual([
      { kind: "region", sketch: 2, region: 1 },
      { kind: "region", sketch: 3, region: 4 },
      { kind: "region", sketch: 5, region: 2 },
    ]);
  });

  it("a click on a grouped solid selects the group; a double-click opens it", () => {
    let entered: number | null = null;
    const I = { linear: [[1, 0, 0], [0, 1, 0], [0, 0, 1]], translation: [0, 0, 0] };
    const graph = {
      context: null,
      path: [],
      nodes: [{ id: 3, kind: "group", name: "Carcass", label: "Carcass", definition: null, parent: null, tag: null, hidden: false, locked: false, visible: true, xform: I, in_component: false }],
      objects: [{ id: 1, parent: 3, hidden: false, locked: false, visible: true }],
      meshes: [],
      sketches: [],
      definitions: [],
      tag_colors: {},
    };
    const h = fakeHost({ _state: { groups: [], graph } as never, enterGroup: (g) => (entered = g) });
    const t = new SelectTool(h);
    t.onEvent(ev("move", [0.5, 0.5, 1], { picked: pickOf(face) }));
    expect(h.hovered).toEqual({ kind: "node", node: 3 });
    t.onEvent(ev("click", [0.5, 0.5, 1], { picked: pickOf(face) }));
    expect(h.sel).toEqual([{ kind: "node", node: 3 }]);
    expect(h.statuses.at(-1)?.text).toContain("group 'Carcass'");
    t.onEvent(ev("dblclick", [0.5, 0.5, 1], { picked: pickOf(face) }));
    expect(entered).toBe(3);
  });

  it("a click outside the open group closes it and selects what was clicked", async () => {
    const I = { linear: [[1, 0, 0], [0, 1, 0], [0, 0, 1]], translation: [0, 0, 0] };
    const graph = {
      context: 3,
      path: [3],
      nodes: [{ id: 3, kind: "group", name: "Carcass", label: "Carcass", definition: null, parent: null, tag: null, hidden: false, locked: false, visible: true, xform: I, in_component: false }],
      objects: [
        { id: 1, parent: null, hidden: false, locked: false, visible: true },
        { id: 2, parent: 3, hidden: false, locked: false, visible: true },
      ],
      meshes: [],
      sketches: [],
      definitions: [],
      tag_colors: {},
    };
    const h = fakeHost({ _state: { groups: [], graph } as never, outcome: { kind: "context_changed", context: null } });
    h.apply = async (op) => {
      h.ops.push(op);
      graph.context = null as never;
      graph.path = [];
      return { outcome: { kind: "context_changed", context: null }, scene: { mesh: null, state: h._state } } as never;
    };
    new SelectTool(h).onEvent(ev("click", [0.5, 0.5, 1], { picked: pickOf(face) }));
    await Promise.resolve();
    await Promise.resolve();
    expect(h.ops).toEqual([{ op: "set_context", node: null }]);
    expect(h.sel).toEqual([face]);
  });

  it("drags a window left to right and a crossing box right to left", () => {
    const h = fakeHost();
    const t = new SelectTool(h);
    t.onEvent(ev("drag", [1, 0, 0], { x: 100, y: 50, start: [10, 10] }));
    expect(t.view().box).toMatchObject({ x0: 10, x1: 100, crossing: false });
    expect(t.busy()).toBe(true);
    t.onEvent(ev("drag", [1, 0, 0], { x: 0, y: 50, start: [10, 10] }));
    expect(t.view().box?.crossing).toBe(true);
    expect(t.cancel()).toBe(true);
    expect(t.view().box).toBeNull();
  });
});

describe("Erase tool", () => {
  it("erases what is clicked through the kernel, and reports refusals", async () => {
    const h = fakeHost();
    const t = new EraseTool(h);
    t.onEvent(ev("click", [0.5, 0, 1], { picked: pickOf(edge) }));
    await Promise.resolve();
    expect(h.ops).toEqual([{ op: "erase_edge", object: 1, a: [0, 0, 1], b: [1, 0, 1] }]);
    // A solid's rim curve is refused here (it would open the solid).
    t.onEvent(ev("click", [0, 0, 1], { picked: pickOf({ kind: "curve", owner: { kind: "object", id: 1 }, curve: 0 }) }));
    expect(h.statuses.at(-1)?.error).toBe(true);
    expect(h.ops).toHaveLength(1);
    // sketchcraft E-50: a point goes to the kernel, which removes it if it is between two
    // edges in line (and says why not otherwise).
    t.onEvent(ev("click", [0, 0, 1], { picked: pickOf({ kind: "vertex", owner: { kind: "object", id: 1 }, at: [0.5, 0, 1] }) }));
    await Promise.resolve();
    expect(h.ops.at(-1)).toEqual({ op: "dissolve_point", object: 1, at: [0.5, 0, 1] });
  });

  it("a drag across edges erases them all as one batch", async () => {
    const h = fakeHost();
    const t = new EraseTool(h);
    const e2: Entity = { ...edge, a: [1, 0, 1], b: [1, 1, 1] } as Entity;
    t.onEvent(ev("drag", [0.5, 0, 1], { picked: pickOf(edge), start: [0, 0] }));
    t.onEvent(ev("drag", [1, 0.5, 1], { picked: pickOf(e2), start: [0, 0] }));
    t.onEvent(ev("drag", [1, 0.6, 1], { picked: pickOf(e2), start: [0, 0] }));
    expect(t.view().overlay[0].points).toHaveLength(4);
    t.onEvent(ev("dragend", [1, 1, 1], { start: [0, 0] }));
    await Promise.resolve();
    expect(h.ops).toHaveLength(1);
    expect(h.ops[0]).toMatchObject({ op: "batch" });
    expect((h.ops[0] as { ops: unknown[] }).ops).toHaveLength(2);
  });
});

describe("Select tool with the selection filter (sketchcraft E-49)", () => {
  // A 1 m square face of solid 1 at z = 1.
  const sq: [number, number, number][] = [
    [0, 0, 1],
    [1, 0, 1],
    [1, 1, 1],
    [0, 1, 1],
  ];
  const mesh = {
    positions: new Float32Array(sq.flat()),
    indices: new Uint32Array([0, 1, 2, 0, 2, 3]),
    faceIds: new Uint32Array([0, 0]),
    edges: new Float32Array(sq.flatMap((p, i) => [...p, ...sq[(i + 1) % 4]])),
    edgeOwner: new Uint32Array([0, 0, 0, 0]),
    edgeOwners: [{ kind: "object", id: 1 }],
    parts: [{ kind: "face", owner: 1, index: 2, origin: [0, 0, 1], normal: [0, 0, 1], u: [1, 0, 0], v: [0, 1, 0] }],
  } as never;

  it("in Points, a click on the face near a corner selects that corner, and says where it is", () => {
    const h = fakeHost({ _mesh: mesh, _units: "m" });
    h.pickFilter = () => "vertex";
    const t = new SelectTool(h);
    t.onEvent(ev("click", [0.9, 0.9, 1], { picked: { entity: face, point: [0.9, 0.9, 1], surface: null } }));
    expect(h.sel).toEqual([{ kind: "vertex", owner: { kind: "object", id: 1 }, at: [1, 1, 1] }]);
    expect(h.statuses.at(-1)?.text).toMatch(/^Selected a point: x .*1.*, y .*1.*, z .*1.*\.$/);
  });

  it("in Faces, a click on an edge selects the face under it", () => {
    const h = fakeHost({ _mesh: mesh });
    h.pickFilter = () => "face";
    const t = new SelectTool(h);
    t.onEvent(ev("click", [0.5, 0, 1], { picked: { entity: edge, point: [0.5, 0, 1], surface: { at: { kind: "face", object: 1, face: 2 }, frame: {} as never, point: [0.5, 0, 1], t: 1 } } }));
    expect(h.sel).toEqual([{ kind: "face", object: 1, face: 2 }]);
  });
});
