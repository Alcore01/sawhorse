import { describe, expect, it } from "vitest";
import type { V3 } from "../kernel/types";
import { facadeAngle, planCentre, wallAngle } from "./align";

/** A w x d box's level edges (floor and roof outline), turned by `a` about the origin. */
function box(w: number, d: number, a: number): [V3, V3][] {
  const turn = ([x, y]: number[], z: number): V3 => [x * Math.cos(a) - y * Math.sin(a), x * Math.sin(a) + y * Math.cos(a), z];
  const c = [[0, 0], [w, 0], [w, d], [0, d]];
  const out: [V3, V3][] = [];
  for (const z of [0, 3]) for (let i = 0; i < 4; i++) out.push([turn(c[i], z), turn(c[(i + 1) % 4], z)]);
  // the uprights do not vote
  for (const p of c) out.push([turn(p, 0), turn(p, 3)]);
  return out;
}

describe("Align to Axes (sketchcraft E-43)", () => {
  it("finds the turn of a building askew, whichever wall is longest", () => {
    for (const deg of [0, 17, -23, 40, 72, 110]) {
      const a = wallAngle(box(12, 5, (deg * Math.PI) / 180))!;
      // modulo 90°: 72° reads as -18°, 110° as 20°.
      const want = ((((deg + 45) % 90) + 90) % 90) - 45;
      expect((a * 180) / Math.PI).toBeCloseTo(want, 9);
    }
  });

  it("goes by the long walls when a short one is crooked", () => {
    const edges = box(20, 10, 0);
    edges.push([[0, 0, 3], [0.5, 0.5, 3]]); // a 0.7 m wall at 45°
    expect(Math.abs((wallAngle(edges)! * 180) / Math.PI)).toBeLessThan(1);
  });

  it("has nothing to go by without level edges, and centres on the plan", () => {
    expect(wallAngle([[[0, 0, 0], [0, 0, 3]]])).toBeNull();
    expect(planCentre(box(4, 2, 0))).toEqual([2, 1, 0]);
  });
});

describe("Align Façade to Axes (sketchcraft E-48)", () => {
  const deg = (r: number | null) => (r === null ? null : (r * 180) / Math.PI);
  const n = (d: number, z = 0): V3 => [Math.cos((d * Math.PI) / 180), Math.sin((d * Math.PI) / 180), z];

  it("turns a façade askew by 20° so it faces red", () => {
    expect(deg(facadeAngle(n(20)))).toBeCloseTo(20, 9);
    expect(deg(facadeAngle(n(200)))).toBeCloseTo(20, 9);
  });

  it("goes the short way, to green when that is nearer", () => {
    expect(deg(facadeAngle(n(110)))).toBeCloseTo(20, 9);
    expect(deg(facadeAngle(n(-70)))).toBeCloseTo(20, 9);
    expect(deg(facadeAngle(n(60)))).toBeCloseTo(-30, 9);
  });

  it("follows the chosen façade, not the longest walls", () => {
    // A façade at 33° in a building whose long walls run at 10°: the façade decides.
    expect(deg(facadeAngle(n(33)))).toBeCloseTo(33, 9);
  });

  it("works on a leaning wall but refuses a roof or floor", () => {
    expect(deg(facadeAngle(n(15, 0.5)))).toBeCloseTo(15, 9);
    expect(facadeAngle([0, 0, 1])).toBeNull();
    expect(facadeAngle([0.05, 0, -1])).toBeNull();
  });
});
