import { describe, expect, it } from "vitest";
import type { DocState } from "../kernel/types";
import { annotationView } from "./annotations";

// E-46: a stored dimension behind the solid must not show through it.
describe("annotationView: dimensions", () => {
  const state = {
    guides: [],
    dimensions: [{ id: 1, a: [0, 0, 0], b: [2, 0, 0], offset: [0, -0.5, 0], length: 2 }],
  } as unknown as DocState;

  it("draws the dimension lines with the depth test, so faces in front hide them", () => {
    const { overlay } = annotationView(state, (m) => `${m}`, []);
    expect(overlay).toHaveLength(1);
    expect(overlay[0].depthTest).toBe(true);
  });

  it("marks the dimension label to be hidden when a face is in front of it", () => {
    const { labels } = annotationView(state, (m) => `${m}`, []);
    expect(labels).toEqual([{ at: [1, -0.5, 0], text: "2", className: "dim", occlude: true }]);
  });
});
