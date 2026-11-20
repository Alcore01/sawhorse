import { describe, expect, it } from "vitest";
import { gizmoDots } from "./axisGizmo";

describe("axes gizmo (sketchcraft E-55)", () => {
  // The Front view: red to the right, blue up, green away from the eye.
  const front = { x: [1, 0, 0] as [number, number, number], y: [0, 0, -1] as [number, number, number], z: [0, 1, 0] as [number, number, number] };

  it("places the dots as the camera sees the axes, the nearest drawn last", () => {
    const d = gizmoDots(front, 30);
    const at = (id: string) => d.find((x) => x.id === id)!;
    expect([at("x").x, at("x").y]).toEqual([30, -0]);
    expect([at("z").x, at("z").y]).toEqual([0, -30]);
    expect(at("y").depth).toBe(-1);
    expect(d[0].id).toBe("y"); // farthest first
    expect(d[d.length - 1].id).toBe("-y"); // the -Y dot faces the eye in the Front view
  });

  it("a click on a dot looks from that side", () => {
    const view = Object.fromEntries(gizmoDots(front, 30).map((x) => [x.id, x.view]));
    expect(view).toEqual({ x: "right", "-x": "left", y: "back", "-y": "front", z: "top", "-z": "bottom" });
  });

  it("only the positive dots carry a letter", () => {
    expect(gizmoDots(front, 30).filter((x) => x.label).map((x) => x.label).sort()).toEqual(["X", "Y", "Z"]);
  });
});
