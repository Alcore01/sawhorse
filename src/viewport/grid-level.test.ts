// sketchcraft E-26 / E-27 (H-20261117-4): the grid level follows the zoom, and the orbit
// pivot never goes to a far ground point.
import { describe, expect, it } from "vitest";
import { GRID_MIN_PX, gridLevel, gridSpec, groundPivotOk } from "./camera";
import { M_PER_IN } from "../lib/units";

describe("gridLevel (E-26)", () => {
  it("keeps the units' grid when its minor cell is wide enough on screen", () => {
    const base = gridSpec("m");
    expect(gridLevel(base, 0.01 / 10)).toBe(base);
  });

  it("steps the metric grid by ten as the view zooms out", () => {
    const base = gridSpec("m");
    const at = (mpp: number) => gridLevel(base, mpp);
    expect(at(0.1 / 10)).toMatchObject({ minor: 0.1, major: 1, label: "100 mm / 1 m" });
    expect(at(1 / 10)).toMatchObject({ minor: 1, major: 10, label: "1 m / 10 m" });
    expect(at(10 / 10)).toMatchObject({ minor: 10, major: 100, label: "10 m / 100 m" });
    expect(at(100 / 10)).toMatchObject({ label: "100 m / 1 km" });
  });

  it("steps the imperial grid to feet, then tens of feet", () => {
    const base = gridSpec("frac-in");
    const ft = 12 * M_PER_IN;
    expect(gridLevel(base, ft / 10)).toMatchObject({ minor: ft, ratio: 10, label: "1 ft / 10 ft" });
    expect(gridLevel(base, (10 * ft) / 10).label).toBe("10 ft / 100 ft");
  });

  it("always leaves a minor cell at least GRID_MIN_PX wide", () => {
    const base = gridSpec("mm");
    for (const mpp of [1e-4, 3e-3, 0.02, 0.7, 5, 42]) {
      expect(gridLevel(base, mpp).minor / mpp).toBeGreaterThanOrEqual(GRID_MIN_PX);
    }
  });
});

describe("groundPivotOk (E-27)", () => {
  it("accepts the ground near the model or the look-at point", () => {
    expect(groundPivotOk([3, 0, 0], [0, 0, 0], 2, 20)).toBe(true);
    expect(groundPivotOk([15, 0, 0], [0, 0, 0], 2, 20)).toBe(true);
  });

  it("refuses a ground point toward the horizon", () => {
    expect(groundPivotOk([400, 0, 0], [0, 0, 0], 2, 20)).toBe(false);
  });
});
