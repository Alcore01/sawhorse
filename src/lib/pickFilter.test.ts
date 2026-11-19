// sketchcraft E-49: the selection filter (Points, Edges, Faces).
import { describe, expect, it } from "vitest";
import type { Entity, MeshData, Picked, V3 } from "../kernel/types";
import { boxFilter, filterPick, meshPoints } from "./pickFilter";

// A 2 m square face (solid 7, face 3) at z = 0, seen from above: 100 px per metre.
const sq: V3[] = [
  [0, 0, 0],
  [2, 0, 0],
  [2, 2, 0],
  [0, 2, 0],
];
const edges = new Float32Array(sq.flatMap((p, i) => [...p, ...sq[(i + 1) % 4]]));
const mesh = {
  positions: new Float32Array(sq.flat()),
  indices: new Uint32Array([0, 1, 2, 0, 2, 3]),
  faceIds: new Uint32Array([0, 0]),
  edges,
  edgeOwner: new Uint32Array([0, 0, 0, 0]),
  edgeOwners: [{ kind: "object", id: 7 }],
  parts: [{ kind: "face", owner: 7, index: 3, origin: [0, 0, 0], normal: [0, 0, 1], u: [1, 0, 0], v: [0, 1, 0] }],
} as unknown as MeshData;
const toScreen = (p: V3): [number, number] => [100 * p[0], 100 * p[1]];
const face: Entity = { kind: "face", object: 7, face: 3 };
const onFace = (point: V3): Picked => ({ entity: face, point, surface: { at: { kind: "face", object: 7, face: 3 }, frame: {} as never, point, t: 1 } });

describe("filterPick", () => {
  it("All keeps what the pick gives", () => {
    expect(filterPick(onFace([1, 1, 0]), "all", mesh, toScreen, [100, 100])).toEqual(face);
  });

  it("Points takes the nearest corner of the solid under the cursor, within 30 px", () => {
    expect(filterPick(onFace([1.8, 0.1, 0]), "vertex", mesh, toScreen, [180, 10])).toEqual({ kind: "vertex", owner: { kind: "object", id: 7 }, at: [2, 0, 0] });
    expect(filterPick(onFace([1, 1, 0]), "vertex", mesh, toScreen, [100, 100])).toBeNull();
  });

  it("Edges takes the nearest edge, Faces the face under an edge", () => {
    const e = filterPick(onFace([1, 0.1, 0]), "edge", mesh, toScreen, [100, 10]);
    expect(e).toMatchObject({ kind: "edge", owner: { kind: "object", id: 7 }, a: [0, 0, 0], b: [2, 0, 0] });
    const edge: Picked = { entity: e as Entity, point: [1, 0, 0], surface: onFace([1, 0, 0]).surface };
    expect(filterPick(edge, "face", mesh, toScreen, [100, 0])).toEqual(face);
  });

  it("nothing under the cursor stays nothing", () => {
    expect(filterPick(null, "vertex", mesh, toScreen, [0, 0])).toBeNull();
  });
});

describe("boxFilter and meshPoints", () => {
  it("a window takes the points inside it, a crossing box the edges it touches", () => {
    const pts = boxFilter(mesh, toScreen, { x0: -10, y0: -10, x1: 210, y1: 50 }, false, "vertex");
    expect(pts.map((p) => (p.kind === "vertex" ? p.at : null))).toEqual([
      [0, 0, 0],
      [2, 0, 0],
    ]);
    const es = boxFilter(mesh, toScreen, { x0: 150, y0: 150, x1: 250, y1: 250 }, true, "edge");
    expect(es).toHaveLength(2);
    expect(boxFilter(mesh, toScreen, { x0: 50, y0: 50, x1: 60, y1: 60 }, true, "face")).toEqual([face]);
  });

  it("lists each point once", () => {
    expect(meshPoints(mesh)).toHaveLength(4);
  });
});
