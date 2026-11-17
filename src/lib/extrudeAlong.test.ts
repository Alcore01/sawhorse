import { describe, expect, it } from "vitest";
import { parseAlong } from "./extrudeAlong";

describe("parseAlong (E-30)", () => {
  it("goes up blue by default", () => {
    expect(parseAlong("30", "cm")).toEqual({ ok: true, vector: [0, 0, 0.3] });
  });
  it("takes an axis first, and a sign", () => {
    expect(parseAlong("green -50", "cm")).toEqual({ ok: true, vector: [0, -0.5, 0] });
    expect(parseAlong("x 1m", "cm")).toEqual({ ok: true, vector: [1, 0, 0] });
  });
  it("refuses nonsense and zero", () => {
    expect(parseAlong("abc", "cm").ok).toBe(false);
    expect(parseAlong("0", "cm").ok).toBe(false);
  });
});
