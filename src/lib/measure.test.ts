// sketchcraft E-10: the selection's area and volume.
import { describe, expect, it } from "vitest";
import type { DocState, Entity, GraphNode, MeshData, MeshPart, SceneGraph } from "../kernel/types";
import { describeMeasure, formatSurface, measureSelection, partAreas } from "./measure";

const part = (kind: MeshPart["kind"], owner: number, index: number): MeshPart => ({ kind, owner, index, origin: [0, 0, 0], normal: [0, 0, 1], u: [1, 0, 0], v: [0, 1, 0] });

// Part 0: face 1 of solid 7, a 2 x 3 m rectangle (two triangles, upright on y = 0).
// Part 1: region 3 of sketch 2, a right triangle of legs 1 m (0.5 m²).
const mesh = {
  positions: new Float32Array([0, 0, 0, 2, 0, 0, 2, 0, 3, 0, 0, 3, 5, 5, 0, 6, 5, 0, 5, 6, 0]),
  indices: new Uint32Array([0, 1, 2, 0, 2, 3, 4, 5, 6]),
  faceIds: new Uint32Array([0, 0, 1]),
  edges: new Float32Array(),
  edgeOwner: new Uint32Array(),
  edgeOwners: [],
  parts: [part("face", 7, 1), part("region", 2, 3)],
} as unknown as MeshData;

const I = { linear: [[1, 0, 0], [0, 1, 0], [0, 0, 1]], translation: [0, 0, 0] } as GraphNode["xform"];
const leaf = (id: number, parent: number | null) => ({ id, parent, hidden: false, locked: false, visible: true, in_component: false });
const graph: SceneGraph = {
  context: null,
  path: [],
  nodes: [{ id: 1, kind: "group", name: "House", label: "House", definition: null, parent: null, tag: null, hidden: false, locked: false, visible: true, xform: I, in_component: false } as GraphNode],
  objects: [leaf(10, 1), leaf(11, 1), leaf(14, null)],
  meshes: [],
  sketches: [],
  definitions: [],
  tag_colors: {},
};
const state = {
  solids: [
    { id: 10, number: 1, min: [0, 0, 0], max: [1, 1, 1], volume: 2 },
    { id: 11, number: 2, min: [0, 0, 0], max: [1, 1, 1], volume: 3 },
    { id: 14, number: 3, min: [0, 0, 0], max: [1, 1, 1], volume: 5 },
  ],
  graph,
} as unknown as DocState;

describe("Selection measures (sketchcraft E-10)", () => {
  it("each part's area comes from its triangles", () => {
    const a = partAreas(mesh);
    expect(a[0]).toBeCloseTo(6, 6);
    expect(a[1]).toBeCloseTo(0.5, 6);
  });

  it("faces and regions add their areas; a group adds its solids' volumes, each solid once", () => {
    const sel: Entity[] = [
      { kind: "face", object: 7, face: 1 },
      { kind: "region", sketch: 2, region: 3 },
      { kind: "node", node: 1 },
      { kind: "object", object: 10 }, // already in the group: counted once
    ];
    const m = measureSelection(sel, state, mesh);
    expect(m.faces).toBe(2);
    expect(m.area).toBeCloseTo(6.5, 6);
    expect(m.solids).toBe(2);
    expect(m.volume).toBeCloseTo(5, 9);
    expect(describeMeasure(m, "cm")).toBe("2 faces · 6.50 m² · 2 solids · 5 m³");
    // A loose solid alone: its volume only.
    expect(describeMeasure(measureSelection([{ kind: "object", object: 14 }], state, mesh), "m")).toBe("1 solid · 5 m³");
    // Nothing measurable (an edge): no text.
    expect(describeMeasure(measureSelection([{ kind: "edge", owner: { kind: "object", id: 7 }, a: [0, 0, 0], b: [1, 0, 0], drawn: false } as Entity], state, mesh), "m")).toBe("");
  });

  it("areas read in square feet with imperial units", () => {
    expect(formatSurface(0.09290304, "ft-in")).toBe("1.00 sq ft");
    expect(formatSurface(12.345, "m")).toBe("12.35 m²");
  });
});
