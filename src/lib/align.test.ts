import { describe, expect, it } from "vitest";
import type { V3 } from "../kernel/types";
import { planCentre, wallAngle } from "./align";

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
