// sketchcraft E-55: the axes in the top right corner of the view (JC, 20 nov. 2026 : « il
// manque une gestion des axes dans le coin supérieur droit de la vue », with Blender's as the
// picture): red X, green Y, blue Z as filled dots with their letter, the opposite directions
// as rings, turning with the camera; a click on one looks from that side.
import type { ViewId } from "../viewport/camera";

type V3 = [number, number, number];

export interface GizmoDot {
  /** "x", "-x", ... */
  id: string;
  /** The standard view looking from that side (eye on that axis). */
  view: ViewId;
  /** Offset from the centre, pixels (y down). */
  x: number;
  y: number;
  /** Toward the eye (larger is nearer): drawn last. */
  depth: number;
  positive: boolean;
  label: string;
  color: string;
}

const AXES: { id: "x" | "y" | "z"; color: string; views: [ViewId, ViewId] }[] = [
  { id: "x", color: "#e0413b", views: ["right", "left"] },
  { id: "y", color: "#6aa84f", views: ["back", "front"] },
  { id: "z", color: "#3d7fe0", views: ["top", "bottom"] },
];

/** The six dots for a camera whose world axes point along `frame` in camera space (x right,
 * y up, z toward the eye), `r` pixels out from the centre; farthest first. */
export function gizmoDots(frame: { x: V3; y: V3; z: V3 }, r: number): GizmoDot[] {
  const out: GizmoDot[] = [];
  for (const a of AXES) {
    const d = frame[a.id];
    for (const s of [1, -1] as const) {
      out.push({
        id: s > 0 ? a.id : `-${a.id}`,
        view: s > 0 ? a.views[0] : a.views[1],
        x: s * d[0] * r,
        y: -s * d[1] * r,
        depth: s * d[2],
        positive: s > 0,
        label: s > 0 ? a.id.toUpperCase() : "",
        color: a.color,
      });
    }
  }
  return out.sort((p, q) => p.depth - q.depth);
}
