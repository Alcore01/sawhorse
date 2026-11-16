// Line (L): click, click, click... each segment is drawn as it is clicked (one undo step
// each) and the chain carries on from its end; it stops when it closes on its start or a
// face forms, or on Esc. Type a length and press Enter to place the next point at that
// distance along the direction the cursor shows (an axis, a parallel, a locked direction).
//
// Planes: a line started inside a face stays on that face's plane; otherwise the plane is
// settled by the first segment (level, or the upright plane containing it). On a solid's
// face a line must cut it from edge to edge (or close into a loop): until it does, the
// stroke stays a preview, then becomes a split_face cut (lane-2 open question 1: yes).
//
// sketchcraft E-18: a chain that did not start inside a face (nor on a plane chosen with the
// arrow keys) is free in space, as in SketchUp: its next point is not held to the plane of
// the segments so far, and a segment that leaves that plane goes onto a new one. When the
// segment before it was alone on its plane, it is drawn again on the new plane with the new
// one (one undo step), so an upright rectangle drawn on from a ground line still closes into
// a face.
import type { EditResult } from "../kernel/client";
import type { V3 } from "../kernel/types";
import { notHere, parseVcb } from "../lib/vcb";
import { add, cross, dist, mul, onPlane, planeAt, planeFromSnap, planeForSegment, sub, unit, type DrawPlane } from "./draw";
import { StrokeTool } from "./stroke";
import { finalTarget, same, snapObject } from "./target";
import type { InferContext, Snap, ToolEvent, ToolView } from "./tool";

export class LineTool extends StrokeTool {
  readonly id = "line" as const;
  /** Points of the current chain (the last one is where the next segment starts). */
  private chain: V3[] = [];
  private snaps: Snap[] = [];
  /** Where the whole chain began (clicking it again closes the chain). */
  private first: V3 | null = null;
  /** A face stroke waiting to reach an edge or close (not in the model yet). */
  private pending = false;
  private viewDir: V3 = [0, 0, -1];
  /** E-18: the chain may change planes (it did not start in a face or on a locked plane). */
  private free = false;
  /** E-18: the point before the chain's last one, and how many segments lie on `plane`. */
  private prev: V3 | null = null;
  private onThisPlane = 0;

  start(): V3 | null {
    return this.chain.length ? this.chain[this.chain.length - 1] : null;
  }

  reset(): void {
    super.reset();
    this.chain = [];
    this.snaps = [];
    this.first = null;
    this.pending = false;
    this.free = false;
    this.prev = null;
    this.onThisPlane = 0;
  }

  valueLabel(): string {
    return "Length";
  }

  inferContext(): InferContext {
    const ctx = super.inferContext();
    const n = this.chain.length;
    // Perpendicular / parallel to the previous segment, as SketchUp users expect.
    if (n >= 2) ctx.refs = [[this.chain[n - 2], this.chain[n - 1]]];
    const s = this.start();
    if (this.prev && s) ctx.refs = [[this.prev, s], ...(ctx.refs ?? [])];
    // E-18: a free chain's next point is not held to the plane of the segments so far.
    if (this.free && this.plane?.target.on !== "face") ctx.plane = null;
    return ctx;
  }

  onEvent(e: ToolEvent): void {
    if (e.type === "move" || e.type === "drag") {
      this.track(e);
      this.viewDir = e.ray.dir;
      const s = this.start();
      if (s && this.cursor) this.host.showValue(this.host.fmt(dist(s, this.cursor)));
      this.host.changed();
      return;
    }
    if (e.type === "click" && e.snap) void this.click(e.snap);
  }

  onValue(text: string): void {
    const h = this.host;
    const r = parseVcb(text, h.units());
    if (!r.ok) return void h.status(r.error, true);
    if (r.value.kind !== "length") return void h.status(notHere(r.value, "Line", "a length"), true);
    const s = this.start();
    if (!s || !this.cursor || !this.snap) return void h.status("Click where the line starts, point the way, then type its length.", true);
    const dir = unit(sub(this.cursor, s));
    if (Math.hypot(...dir) < 0.5) return void h.status("Move the cursor to show the direction first.", true);
    const end = add(s, mul(dir, r.value.m));
    void this.click({ ...this.snap, point: end });
  }

  private async click(snap: Snap): Promise<void> {
    const h = this.host;
    const p = snap.point;
    if (!this.chain.length) {
      this.chain = [p];
      this.snaps = [snap];
      this.first = p;
      // Inside a face: the line stays on its plane. Elsewhere the first segment decides.
      this.plane = snap.kind === "on-face" ? planeFromSnap(snap, this.prefer) : this.prefer !== "auto" ? planeFromSnap(snap, this.prefer) : null;
      this.free = this.plane === null;
      this.prev = null;
      this.onThisPlane = 0;
      h.status(`Line started${this.plane ? ` ${this.plane.where(h.fmt)}` : ""}. Click the next point or type a length.`);
      h.changed();
      return;
    }
    const last = this.chain[this.chain.length - 1];
    if (same(p, last)) return;
    if (!this.plane) this.plane = this.settlePlane(this.snaps[0], snap, last, p);
    if (this.free && this.plane.target.on !== "face" && !onPlane(this.plane.frame, p, 1e-6)) {
      await this.turn(last, snap);
      return;
    }
    if (!onPlane(this.plane.frame, p, 1e-6)) {
      h.status("That point is off this line's plane. Esc starts a new line.", true);
      return;
    }
    this.chain.push(p);
    this.snaps.push(snap);
    const closes = !!this.first && same(p, this.first) && !same(last, this.first);
    const target = finalTarget(this.plane, this.snaps);
    if (target.target.on === "face") {
      await this.faceStroke(target, closes);
      return;
    }
    const seg: V3[] = [last, p];
    const r = await this.stroke(target.target, seg, false, (res: EditResult) => `Line ${h.fmt(dist(last, p))} ${h.unitLabel()} ${target.where(h.fmt)}${formed(res) ? ": a face formed" : ""}.`);
    if (!r) {
      this.chain.pop();
      this.snaps.pop();
      return;
    }
    // After the first segment on a free plane, later segments stay on it (as a sketch).
    if (this.plane.target.on === "face") this.plane = target;
    this.after(last, p, snap, closes || formed(r), 1);
  }

  /** The chain carries on from `p` (or ends); `added` segments went on `plane`. */
  private after(last: V3, p: V3, snap: Snap, ends: boolean, added: number): void {
    if (ends) this.reset();
    else {
      this.chain = [p];
      this.snaps = [snap];
      this.prev = last;
      this.onThisPlane += added;
    }
    this.host.changed();
  }

  /**
   * E-18: the next segment leaves the plane of a free chain. With the segment before it
   * alone on that plane (and not in line with it), both go onto the plane they share: that
   * segment is undone and drawn again with the new one, so a loop can still close into a
   * face. Otherwise the new segment settles a plane of its own.
   */
  private async turn(last: V3, snap: Snap): Promise<void> {
    const h = this.host;
    const p = snap.point;
    const prev = this.prev;
    const n = prev ? cross(sub(last, prev), sub(p, last)) : null;
    const shared = !!prev && !!n && Math.hypot(...n) > 1e-9 * dist(prev, last) * dist(last, p) && this.onThisPlane === 1;
    const plane = shared ? planeAt(last, n!) : this.settlePlane(this.snaps[this.snaps.length - 1], snap, last, p);
    const closes = !!this.first && same(p, this.first) && !same(last, this.first);
    if (shared && prev) {
      if (!(await h.apply({ op: "undo" } as never))) return;
      this.plane = plane;
      const r = await this.stroke(plane.target, [prev, last, p], false, (res: EditResult) => `Line ${h.fmt(dist(last, p))} ${h.unitLabel()} ${plane.where(h.fmt)} (with the segment before it)${formed(res) ? ": a face formed" : ""}.`);
      if (!r) return void h.changed();
      this.onThisPlane = 0;
      this.after(last, p, snap, closes || formed(r), 2);
      return;
    }
    this.plane = plane;
    this.onThisPlane = 0;
    const r = await this.stroke(plane.target, [last, p], false, (res: EditResult) => `Line ${h.fmt(dist(last, p))} ${h.unitLabel()} ${plane.where(h.fmt)}${formed(res) ? ": a face formed" : ""}.`);
    if (!r) return void h.changed();
    this.after(last, p, snap, closes || formed(r), 1);
  }

  /** A stroke on a solid's face: commit when it reaches an edge or closes, else keep it. */
  private async faceStroke(target: DrawPlane, closes: boolean): Promise<void> {
    const h = this.host;
    const pts = closes ? this.chain.slice(0, -1) : this.chain;
    const r = await h.apply(
      { op: "draw", target: target.target, points: pts, closed: closes } as never,
      (res) => `Cut ${target.where(h.fmt)}: ${res.outcome.kind === "drawn" ? res.outcome.faces.length : 0} faces now.`,
    );
    if (r) {
      this.reset();
      h.changed();
      return;
    }
    // A dangling stroke is not an error yet, just not finished; anything else is.
    if ((h.lastError() ?? "").includes("must start and end on its edges")) {
      this.pending = true;
      h.status("Line on a face: carry on to an edge of the face, or close the loop. Esc drops it.", false);
    } else {
      this.chain.pop();
      this.snaps.pop();
    }
    h.changed();
  }

  /** The plane of a line that did not start inside a face. */
  private settlePlane(first: Snap, second: Snap, a: V3, b: V3): DrawPlane {
    for (const s of [first, second]) {
      if (s.surface?.at.kind === "face" || s.surface?.at.kind === "region") {
        const cand = planeFromSnap(s);
        if (onPlane(cand.frame, a) && onPlane(cand.frame, b)) {
          // Both ends on one solid's face plane, on that solid: draw on the face.
          if (cand.target.on === "face" && snapObject(first, cand) !== cand.target.object) continue;
          return cand;
        }
      }
    }
    return planeForSegment(a, b, this.viewDir);
  }

  view(): ToolView {
    const s = this.start();
    const extra = [];
    if (this.chain.length > 1 || this.pending) extra.push({ kind: "strip" as const, points: this.chain, color: "--sel", width: 2 });
    if (s && this.cursor && !same(s, this.cursor)) extra.push({ kind: "strip" as const, points: [s, this.cursor], color: this.bandColor(), width: 2 });
    return this.baseView(extra, s && this.cursor ? this.live(this.host.fmt(dist(s, this.cursor))) : []);
  }
}

function formed(r: EditResult): boolean {
  return r.outcome.kind === "drawn" && r.outcome.regions.length > 0;
}
