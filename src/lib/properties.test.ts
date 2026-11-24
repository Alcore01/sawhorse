// sketchcraft E-56: the Info pane's measures.
import { describe, expect, it } from "vitest";
import type { MeshData, V3 } from "../kernel/types";
import { azimuth, faceKind, objectVolume, partAreas, selectionInfo, slope } from "./properties";

/** A 2 × 3 × 4 m box (solid 7), one part per side, outward normals. */
function box(): MeshData {
  const [X, Y, Z] = [2, 3, 4];
  const quads: { n: V3; q: V3[] }[] = [
    { n: [0, 0, -1], q: [[0, 0, 0], [0, Y, 0], [X, Y, 0], [X, 0, 0]] },
    { n: [0, 0, 1], q: [[0, 0, Z], [X, 0, Z], [X, Y, Z], [0, Y, Z]] },
    { n: [0, -1, 0], q: [[0, 0, 0], [X, 0, 0], [X, 0, Z], [0, 0, Z]] },
    { n: [0, 1, 0], q: [[0, Y, 0], [0, Y, Z], [X, Y, Z], [X, Y, 0]] },
    { n: [-1, 0, 0], q: [[0, 0, 0], [0, 0, Z], [0, Y, Z], [0, Y, 0]] },
    { n: [1, 0, 0], q: [[X, 0, 0], [X, Y, 0], [X, Y, Z], [X, 0, Z]] },
  ];
  const pos: number[] = [];
  const idx: number[] = [];
  const ids: number[] = [];
  quads.forEach((f, i) => {
    const b = pos.length / 3;
    f.q.forEach((p) => pos.push(...p));
    idx.push(b, b + 1, b + 2, b, b + 2, b + 3);
    ids.push(i, i);
  });
  return {
    positions: new Float32Array(pos),
    indices: new Uint32Array(idx),
    faceIds: new Uint32Array(ids),
    edges: new Float32Array(),
    edgeOwner: new Uint32Array(),
    edgeOwners: [],
    parts: quads.map((f, i) => ({ kind: "face", owner: 7, index: i + 1, origin: f.q[0], normal: f.n, u: [1, 0, 0], v: [0, 1, 0] })),
  } as unknown as MeshData;
}

describe("Info pane measures (sketchcraft E-56)", () => {
  it("areas and volume of a box", () => {
    const m = box();
    expect(partAreas(m)).toEqual([6, 6, 8, 8, 12, 12]);
    expect(objectVolume(m, 7)).toBeCloseTo(24, 9);
  });

  it("kinds, orientation from north (green) and slope", () => {
    expect(faceKind([0, -1, 0])).toBe("wall");
    expect(faceKind([0, 0, 1])).toBe("roof");
    expect(faceKind([0.5, 0, 0.866])).toBe("roof");
    expect(faceKind([0, 0, -1])).toBe("floor");
    expect(azimuth([0, -1, 0])).toEqual({ deg: 180, point: "S" });
    expect(azimuth([1, 0, 0]).point).toBe("E");
    expect(azimuth([-1, 1, 0]).point).toBe("NW");
    expect(slope([0.5, 0, Math.sqrt(3) / 2])).toBeCloseTo(30, 6);
  });

  it("a solid: its volume and envelope split into walls, roofs and floor", () => {
    const s = selectionInfo([{ kind: "object", object: 7 }], box(), null, "m");
    const v = Object.fromEntries(s.rows.map((r) => [r.label, r.value]));
    expect(v).toEqual({ Volume: "24.000 m³", Envelope: "52.00 m²", Walls: "40.00 m²", Roofs: "6.00 m²", Floor: "6.00 m²" });
  });

  it("a wall facing south: its area and orientation", () => {
    const s = selectionInfo([{ kind: "face", object: 7, face: 3 }], box(), null, "m");
    const v = Object.fromEntries(s.rows.map((r) => [r.label, r.value]));
    expect(v).toEqual({ Area: "8.00 m²", Kind: "Wall", Facing: "S · 180.0° from north" });
  });

  it("several faces add up; a point gives its coordinates", () => {
    const s = selectionInfo(
      [
        { kind: "face", object: 7, face: 3 },
        { kind: "face", object: 7, face: 4 },
      ],
      box(),
      null,
      "m",
    );
    expect(s.rows.find((r) => r.label === "Total area")?.value).toBe("16.00 m²");
    const p = selectionInfo([{ kind: "vertex", owner: { kind: "object", id: 7 }, at: [2, 3, 4] }], box(), null, "m");
    expect(p.rows.map((r) => r.value)).toEqual(["2", "3", "4"]);
  });
});

describe("orientation with north turned (sketchcraft E-62)", () => {
  it("a wall facing green faces north when north is along green, west when north is 90° clockwise", () => {
    expect(azimuth([0, 1, 0], 0).point).toBe("N");
    expect(azimuth([0, 1, 0], 90).point).toBe("W");
    expect(azimuth([1, 0, 0], 90)).toEqual({ deg: 0, point: "N" });
  });
});
