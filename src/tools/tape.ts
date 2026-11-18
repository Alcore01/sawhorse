// Tape measure (T) and Dimension (D).
//
// Tape: click two points to measure between them (nothing is drawn). Click an edge (not
// its end) and move away from it to place a guide line parallel to it; click, or type the
// offset. From a point, a typed length drops a guide point that far along the direction
// the cursor shows. Guides are inferable: shapes snap to them and to where they cross
// edges (the dado layout on a bookshelf side).
//
// Dimension: click two points (or one edge), then click where the dimension line goes.
// The text is the length in the display units. sketchcraft E-45: the line's offset snaps
// (within 20°) to the plane of the faces the points were picked on (the line lies in the
// face, beside the measured edge) or square to them (it stands out of the face), and to red,
// green and blue; the status line says which.
import type { V3 } from "../kernel/types";
import { notHere, parseVcb } from "../lib/vcb";
import { add, dist, dot, len, mul, sub, unit } from "./draw";
import { StrokeTool } from "./stroke";
import type { Snap, ToolEvent, ToolView } from "./tool";

/** The perpendicular from the line through `a` and `b` to `p`. */
export function perpendicularFrom(a: V3, b: V3, p: V3): V3 {
  const d = unit(sub(b, a));
  const v = sub(p, a);
  return sub(v, mul(d, dot(v, d)));
}

function edgeOf(s: Snap): [V3, V3] | null {
  const e = s.entity;
  if (s.kind === "on-edge" && e && e.kind === "edge") return [e.a, e.b];
  return null;
}

export class TapeTool extends StrokeTool {
  readonly id = "tape" as const;
  private a: V3 | null = null;
  /** Guide mode: the edge the tape started on. */
  private edge: [V3, V3] | null = null;

  start(): V3 | null {
    return this.a;
  }

  reset(): void {
    super.reset();
    this.a = null;
    this.edge = null;
  }

  valueLabel(): string {
    return this.edge ? "Offset" : "Length";
  }

  onEvent(e: ToolEvent): void {
    const h = this.host;
    if (e.type === "move" || e.type === "drag") {
      this.track(e);
      if (this.a && this.cursor) h.showValue(h.fmt(this.measure()));
      h.changed();
      return;
    }
    if (e.type !== "click" || !e.snap) return;
    const s = e.snap;
    if (!this.a) {
      this.a = s.point;
      this.edge = edgeOf(s);
      h.status(
        this.edge
          ? "Guide from this edge: move away from it and click, or type the offset."
          : "Measuring from here: click the second point. Type a length to drop a guide point.",
      );
      h.changed();
      return;
    }
    if (this.edge) {
      const off = perpendicularFrom(this.edge[0], this.edge[1], s.point);
      if (len(off) < 1e-9) return;
      void this.placeGuide(off);
      return;
    }
    const d = dist(this.a, s.point);
    const delta = sub(s.point, this.a);
    h.status(`Distance ${h.fmt(d)} ${h.unitLabel()} (x ${h.fmt(Math.abs(delta[0]))}, y ${h.fmt(Math.abs(delta[1]))}, z ${h.fmt(Math.abs(delta[2]))}).`);
    h.showValue(h.fmt(d));
    this.reset();
    h.changed();
  }

  private measure(): number {
    if (!this.a || !this.cursor) return 0;
    if (this.edge) return len(perpendicularFrom(this.edge[0], this.edge[1], this.cursor));
    return dist(this.a, this.cursor);
  }

  private async placeGuide(offset: V3): Promise<void> {
    const h = this.host;
    const e = this.edge;
    if (!e) return;
    const dir = unit(sub(e[1], e[0]));
    this.reset();
    h.changed();
    await h.apply({ op: "add_guide", point: add(e[0], offset), dir }, () => `Guide ${h.fmt(len(offset))} ${h.unitLabel()} from the edge. Shapes snap to it and to where it crosses edges.`);
  }

  onValue(text: string): void {
    const h = this.host;
    const r = parseVcb(text, h.units());
    if (!r.ok) return void h.status(r.error, true);
    if (r.value.kind !== "length") return void h.status(notHere(r.value, "Tape", "a length"), true);
    const L = r.value.m;
    if (!this.a) return void h.status("Click an edge (for a guide line) or a point (to measure) first.", true);
    if (this.edge) {
      const cur = this.cursor ? perpendicularFrom(this.edge[0], this.edge[1], this.cursor) : null;
      if (!cur || len(cur) < 1e-9) return void h.status("Move the cursor to the side the guide goes, then type the offset.", true);
      void this.placeGuide(mul(unit(cur), L));
      return;
    }
    if (!this.cursor || dist(this.cursor, this.a) < 1e-9) return void h.status("Point the way first, then type the length.", true);
    const p = add(this.a, mul(unit(sub(this.cursor, this.a)), L));
    this.reset();
    h.changed();
    void h.apply({ op: "add_guide", point: p, dir: null }, () => `Guide point ${h.fmt(Math.abs(L))} ${h.unitLabel()} along.`);
  }

  view(): ToolView {
    const a = this.a;
    const c = this.cursor;
    if (!a || !c) return this.baseView();
    if (this.edge) {
      const off = perpendicularFrom(this.edge[0], this.edge[1], c);
      const dir = unit(sub(this.edge[1], this.edge[0]));
      const p = add(this.edge[0], off);
      return this.baseView(
        [
          { kind: "segments", points: [add(p, mul(dir, -2)), add(p, mul(dir, 2))], color: "--inf-guide", width: 1.5, dashed: true },
          { kind: "segments", points: [add(c, mul(off, -1)), c], color: "--sel", width: 1.25 },
        ],
        this.live(this.host.fmt(len(off))),
      );
    }
    return this.baseView([{ kind: "segments", points: [a, c], color: this.bandColor(), width: 1.5, dashed: true }], this.live(this.host.fmt(dist(a, c))));
  }
}

/** A direction the dimension line may stand off along, and how to say it. */
interface OffsetWay {
  dir: V3;
  how: string;
}

const AXES: [V3, string][] = [
  [[1, 0, 0], "along red"],
  [[0, 1, 0], "along green"],
  [[0, 0, 1], "along blue"],
];

function cross(a: V3, b: V3): V3 {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}

/** sketchcraft E-45: the directions square to the span `a`-`b` a dimension line may stand off
 * along: in each source face's plane and square to it, then the axes; parallel ones once. */
export function offsetWays(a: V3, b: V3, normals: readonly V3[]): OffsetWay[] {
  const d = unit(sub(b, a));
  const ways: OffsetWay[] = [];
  const add1 = (v: V3, how: string) => {
    const w = sub(v, mul(d, dot(v, d)));
    if (len(w) < 0.1) return;
    const u = unit(w);
    if (ways.some((x) => Math.abs(dot(x.dir, u)) > 1 - 1e-9)) return;
    ways.push({ dir: u, how });
  };
  for (const n of normals) {
    add1(cross(n, d), "in the face's plane");
    add1(n, "square to the face");
  }
  for (const [axis, how] of AXES) add1(axis, how);
  return ways;
}

/** The offset to `p` from the span `a`-`b`, snapped to the nearest of `ways` within 20°. */
export function snappedOffset(a: V3, b: V3, p: V3, ways: readonly OffsetWay[]): { offset: V3; how: string | null } {
  const raw = perpendicularFrom(a, b, p);
  const l = len(raw);
  if (l < 1e-12) return { offset: raw, how: null };
  let best: OffsetWay | null = null;
  let bestCos = Math.cos((20 * Math.PI) / 180);
  for (const w of ways) {
    const c = Math.abs(dot(raw, w.dir)) / l;
    if (c >= bestCos) {
      bestCos = c;
      best = w;
    }
  }
  if (!best) return { offset: raw, how: null };
  return { offset: mul(best.dir, dot(raw, best.dir)), how: best.how };
}

export class DimensionTool extends StrokeTool {
  readonly id = "dim" as const;
  private a: V3 | null = null;
  private b: V3 | null = null;
  /** sketchcraft E-45: the normals of the faces the points were picked on. */
  private normals: V3[] = [];
  private how: string | null = null;

  start(): V3 | null {
    return this.b ?? this.a;
  }

  reset(): void {
    super.reset();
    this.a = null;
    this.b = null;
    this.normals = [];
    this.how = null;
  }

  private placed(c: V3): { offset: V3; how: string | null } {
    const a = this.a as V3;
    const b = this.b as V3;
    return snappedOffset(a, b, c, offsetWays(a, b, this.normals));
  }

  onEvent(e: ToolEvent): void {
    const h = this.host;
    if (e.type === "move" || e.type === "drag") {
      this.track(e);
      if (this.a && this.b && this.cursor) {
        const { how } = this.placed(this.cursor);
        if (how !== this.how) {
          this.how = how;
          h.status(how ? `Dimension line ${how}. Click to place it.` : "Click where the dimension line goes (it snaps to the faces' planes, square to them, and to the axes).");
        }
      }
      h.changed();
      return;
    }
    if (e.type !== "click" || !e.snap) return;
    const s = e.snap;
    const n = e.picked?.surface?.frame.normal;
    if (n && !this.b) this.normals.push(n);
    if (!this.a) {
      const edge = edgeOf(s);
      if (edge) {
        [this.a, this.b] = edge;
        h.status("Dimensioning that edge: click where the dimension line goes.");
      } else {
        this.a = s.point;
        h.status("Click the second point.");
      }
      h.changed();
      return;
    }
    if (!this.b) {
      if (dist(this.a, s.point) < 1e-9) return;
      this.b = s.point;
      h.status("Click where the dimension line goes.");
      h.changed();
      return;
    }
    const a = this.a;
    const b = this.b;
    const { offset } = this.placed(s.point);
    this.reset();
    h.changed();
    void h.apply({ op: "add_dimension", a, b, offset }, () => `Dimension ${h.fmt(dist(a, b))} ${h.unitLabel()}.`);
  }

  view(): ToolView {
    const a = this.a;
    const c = this.cursor;
    if (!a || !c) return this.baseView();
    if (!this.b) return this.baseView([{ kind: "segments", points: [a, c], color: "--accent", width: 1.25, dashed: true }], this.live(this.host.fmt(dist(a, c))));
    const off = this.placed(c).offset;
    const a2 = add(a, off);
    const b2 = add(this.b, off);
    return this.baseView(
      [{ kind: "segments", points: [a, a2, this.b, b2, a2, b2], color: "--accent", width: 1.25 }],
      [{ at: mul(add(a2, b2), 0.5), text: this.host.fmt(dist(a, this.b)), className: "dim" }],
    );
  }
}
