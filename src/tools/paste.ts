// sketchcraft E-58: Paste (Edit > Paste, Ctrl+V). JC, 22 nov. 2026 : « j'ai tenter un copié
// collé de la base du polygone d'un batiment et rien aucune possibilité de le copié de de le
// déplacer sur la map ». What Edit > Copy (Ctrl+C) took follows the cursor, held by the lowest
// corner of what was copied, and a click puts it there (snapping as drawing does; a typed
// distance is not taken). Solids, groups and components are copied whole; the faces of a solid
// and loose faces come as their outline, drawn as a flat face on their plane where they land;
// edges as lines. One undo step; then back to Select with nothing in hand.
import type { KernelOp, V3 } from "../kernel/types";
import { translation } from "./xform";
import { EMPTY_VIEW, Tool, markerOf, type Snap, type ToolEvent, type ToolView } from "./tool";

export interface Clip {
  objects: number[];
  nodes: number[];
  /** Outlines of faces (world rings, outer first) and the plane's normal. */
  faces: { rings: V3[][]; normal: V3 }[];
  /** Loose edges. */
  segments: [V3, V3][];
  /** What the preview draws (every edge of what was copied). */
  preview: [V3, V3][];
  /** The point carried by the cursor: the lowest corner of what was copied. */
  anchor: V3;
  /** "2 solids", "the outline of a face"… for the status line. */
  what: string;
}

const add = (a: V3, d: V3): V3 => [a[0] + d[0], a[1] + d[1], a[2] + d[2]];

/** The lowest corner of a set of points: least z, then least x and y (a building's footprint
 * corner on the ground). */
export function anchorOf(points: V3[]): V3 {
  if (!points.length) return [0, 0, 0];
  let [x, y, z] = [Infinity, Infinity, Infinity];
  for (const p of points) {
    x = Math.min(x, p[0]);
    y = Math.min(y, p[1]);
    z = Math.min(z, p[2]);
  }
  return [x, y, z];
}

/** A plane through a line, for drawing it: upright through a level line, else through red. */
function lineNormal(a: V3, b: V3): V3 {
  const d: V3 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
  const h = Math.hypot(d[0], d[1]);
  if (h < 1e-9) return [1, 0, 0];
  return Math.abs(d[2]) < 1e-9 ? [0, 0, 1] : [-d[1] / h, d[0] / h, 0];
}

/** The ops that put a copy of `clip` moved by `d` (one batch). */
export function pasteOps(clip: Clip, d: V3): KernelOp[] {
  const ops: KernelOp[] = [];
  if (clip.objects.length || clip.nodes.length) ops.push({ op: "copy", objects: clip.objects, xform: translation(d), ...(clip.nodes.length ? { nodes: clip.nodes } : {}) });
  for (const f of clip.faces)
    for (const r of f.rings) if (r.length >= 3) ops.push({ op: "draw", target: { on: "plane", origin: add(r[0], d), normal: f.normal }, points: r.map((p) => add(p, d)), closed: true });
  for (const [a, b] of clip.segments) ops.push({ op: "draw", target: { on: "plane", origin: add(a, d), normal: lineNormal(a, b) }, points: [add(a, d), add(b, d)], closed: false });
  return ops;
}

export class PasteTool extends Tool {
  readonly id = "paste" as const;
  private clip: Clip | null = null;
  private snap: Snap | null = null;

  /** What the next placement puts down. */
  setClip(c: Clip | null): void {
    this.clip = c;
    this.snap = null;
  }

  activate(): void {
    this.host.status(this.clip ? `Pasting ${this.clip.what}: click where its lowest corner goes (Esc cancels).` : "Nothing to paste: select something and Edit > Copy (Ctrl+C) first.", !this.clip);
  }

  reset(): void {
    this.snap = null;
  }

  /** Esc: nothing is pasted, back to Select. */
  cancel(): boolean {
    this.snap = null;
    this.host.status("Paste cancelled.");
    this.host.done?.();
    return true;
  }

  onEvent(e: ToolEvent): void {
    if (!this.clip) return;
    if (e.type === "move") {
      this.snap = e.snap;
      this.host.changed();
      return;
    }
    if (e.type === "click" && e.snap) {
      const c = this.clip;
      const to = e.snap.point;
      const d: V3 = [to[0] - c.anchor[0], to[1] - c.anchor[1], to[2] - c.anchor[2]];
      const ops = pasteOps(c, d);
      if (!ops.length) return;
      void this.host.apply(ops.length === 1 ? ops[0] : { op: "batch", ops }, () => `Pasted ${c.what}.`).then((r) => {
        if (r) this.host.done?.();
      });
    }
  }

  view(): ToolView {
    const c = this.clip;
    const s = this.snap;
    if (!c || !s) return EMPTY_VIEW;
    const d: V3 = [s.point[0] - c.anchor[0], s.point[1] - c.anchor[1], s.point[2] - c.anchor[2]];
    return {
      ...EMPTY_VIEW,
      overlay: [{ kind: "segments", points: c.preview.flatMap(([a, b]) => [add(a, d), add(b, d)]), color: "--ok", width: 1.5 }],
      marker: markerOf(s),
    };
  }
}
