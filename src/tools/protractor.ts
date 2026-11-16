// Protractor (sketchcraft, E-01). Places an angled guide line, as SketchUp's Protractor does:
// the roof pitch, a cut corner, a wall off square.
//
// The protractor sits on the plane under the cursor (a face's plane, or level on the
// ground); the arrow keys lock it upright facing red (Right), facing green (Left) or level
// (Up), and holding Shift keeps the plane it is on while the cursor moves away (the same
// keys as Rotate). Click the vertex, click along the reference line (an edge, an axis),
// then click where the guide goes, or type its angle: degrees (35, -22.5, 30°), a slope
// in percent (70%) or a rise and run (1:2). The cursor's angle snaps to the 15 degree
// ticks when it is near one; a typed angle is exact. The guide goes through the vertex,
// on the side of the reference the cursor is on.
import type { V3 } from "../kernel/types";
import { INFERENCE_STYLES } from "../lib/inferenceStyle";
import { add, alignedFrame, dist, dot, mul, sub, unit } from "./draw";
import { AXIS_KEYS, TICK_DEG, axisWords, fmtDeg, parseAngle, rotate, snapTurn } from "./rotate";
import { EMPTY_VIEW, Tool, inferenceLine, markerOf, type InferContext, type Snap, type ToolEvent, type ToolView } from "./tool";
import { projectToPlane, signedAngle } from "./xform";

const DEG = Math.PI / 180;

/**
 * A typed protractor angle, in degrees: "35", "-22.5", "30°", "30deg"; a slope in percent,
 * "70%" (rise over run x 100); or a rise and run, "1:2" (also "1/2" is a length elsewhere,
 * so only the colon form). Null when the text is none of these or the slope is not finite.
 */
export function parseProtractorAngle(text: string): number | null {
  const deg = parseAngle(text);
  if (deg !== null) return deg;
  const pct = /^\s*(-?(?:\d+(?:\.\d*)?|\.\d+))\s*%\s*$/.exec(text);
  if (pct) return Math.atan(Number(pct[1]) / 100) / DEG;
  const ratio = /^\s*(-?(?:\d+(?:\.\d*)?|\.\d+))\s*:\s*((?:\d+(?:\.\d*)?|\.\d+))\s*$/.exec(text);
  if (ratio) {
    const run = Number(ratio[2]);
    if (!(run > 0)) return null;
    return Math.atan(Number(ratio[1]) / run) / DEG;
  }
  return null;
}

/** "35° (70%)": the angle and, under 90 degrees, its slope. */
export function fmtSlope(a: number): string {
  const d = Math.abs(a / DEG);
  if (d >= 90 - 1e-9) return fmtDeg(a);
  const pct = Math.round(Math.tan(Math.abs(a)) * 1000) / 10;
  return `${fmtDeg(a)} (${pct}%)`;
}

interface Gesture {
  vertex: V3;
  normal: V3;
  /** Unit direction of the reference line, once clicked. */
  ref: V3 | null;
  /** Current angle (radians, signed about `normal`). */
  angle: number;
  cursor: V3 | null;
}

export class ProtractorTool extends Tool {
  readonly id = "protractor" as const;
  private snap: Snap | null = null;
  private g: Gesture | null = null;
  private axisLock: (typeof AXIS_KEYS)[string] | null = null;
  private heldNormal: V3 | null = null;
  private hoverNormal: V3 = [0, 0, 1];
  private hoverAt: V3 | null = null;

  valueLabel(): string {
    return "Angle";
  }

  busy(): boolean {
    return this.g !== null;
  }

  reset(): void {
    this.g = null;
  }

  activate(): void {
    this.host.status("Click the vertex (arrow keys choose the protractor's plane), then along the reference line, then where the guide goes; or type an angle, 70% or 1:2.");
  }

  private normalFor(snap: Snap | null): V3 {
    if (this.axisLock) return this.axisLock.n;
    if (this.heldNormal) return this.heldNormal;
    const s = snap?.surface ?? snap?.adjacent ?? null;
    return s ? s.frame.normal : [0, 0, 1];
  }

  inferContext(): InferContext {
    const g = this.g;
    if (!g) return {};
    return { start: g.vertex, plane: { origin: g.vertex, normal: g.normal } };
  }

  onKey(e: KeyboardEvent): boolean {
    const k = AXIS_KEYS[e.key];
    if (!k) return false;
    if (this.g) {
      this.host.status("The protractor's plane is set at the vertex; Esc and start again to change it.", true);
      return true;
    }
    this.axisLock = this.axisLock?.kind === k.kind ? null : k;
    this.host.status(this.axisLock ? `Protractor locked ${this.axisLock.name}. Press the same arrow to unlock.` : "Protractor follows the face under the cursor.");
    this.hoverNormal = this.normalFor(this.snap);
    this.host.changed();
    return true;
  }

  onShift(down: boolean): void {
    if (this.g) return;
    this.heldNormal = down ? this.normalFor(this.snap) : null;
    this.host.changed();
  }

  private onPlane(e: ToolEvent, g: Gesture): V3 | null {
    if (e.snap && Math.abs(dot(sub(e.snap.point, g.vertex), g.normal)) < 1e-7) return e.snap.point;
    const den = dot(e.ray.dir, g.normal);
    if (Math.abs(den) > 1e-12) {
      const t = dot(sub(g.vertex, e.ray.origin), g.normal) / den;
      if (t > 0) return add(e.ray.origin, mul(e.ray.dir, t));
    }
    return e.snap ? projectToPlane(e.snap.point, g.vertex, g.normal) : null;
  }

  onEvent(e: ToolEvent): void {
    const h = this.host;
    if (e.type === "move" || e.type === "drag") {
      this.snap = e.snap;
      const g = this.g;
      if (!g) {
        this.hoverNormal = this.normalFor(e.snap);
        this.hoverAt = e.snap?.point ?? null;
      } else {
        g.cursor = this.onPlane(e, g);
        if (g.ref && g.cursor && dist(g.cursor, g.vertex) > 1e-9) {
          g.angle = snapTurn(signedAngle(g.ref, unit(sub(g.cursor, g.vertex)), g.normal)).angle;
          h.showValue(fmtDeg(g.angle));
        }
      }
      h.changed();
      return;
    }
    if (e.type !== "click" || !e.snap) return;
    const g = this.g;
    if (!g) {
      const normal = unit(this.normalFor(e.snap));
      this.g = { vertex: e.snap.point, normal, ref: null, angle: 0, cursor: null };
      h.status(`Protractor on a plane square to ${axisWords(normal)}: click along the reference line.`);
      h.changed();
      return;
    }
    const p = this.onPlane(e, g);
    if (!g.ref) {
      if (!p || dist(p, g.vertex) < 1e-6) return;
      g.ref = unit(sub(p, g.vertex));
      g.cursor = p;
      h.status("Now click where the guide goes, or type its angle (35, 70% or 1:2).");
      h.changed();
      return;
    }
    if (Math.abs(g.angle) < 1e-9) return;
    void this.place(g.angle);
  }

  onValue(text: string): void {
    const h = this.host;
    const deg = parseProtractorAngle(text);
    if (deg === null) return void h.status("Protractor takes an angle in degrees (35, -22.5), a slope (70%) or a rise and run (1:2).", true);
    const g = this.g;
    if (!g) return void h.status("Click the vertex and the reference line first, then type the angle.", true);
    if (!g.ref) g.ref = alignedFrame(g.vertex, g.normal).u;
    // The side of the reference the cursor is on (the sign of its angle); a negative typed
    // angle turns the other way from there.
    const sign = g.angle < 0 ? -1 : 1;
    void this.place(sign * deg * DEG);
  }

  private async place(angle: number): Promise<void> {
    const h = this.host;
    const g = this.g;
    if (!g || !g.ref) return;
    const dir = unit(rotate(g.ref, g.normal, angle));
    const point = g.vertex;
    this.reset();
    h.changed();
    h.showValue(fmtDeg(angle));
    await h.apply({ op: "add_guide", point, dir }, () => `Guide at ${fmtSlope(angle)} from the reference line. Shapes snap to it and to where it crosses edges.`);
  }

  view(): ToolView {
    const g = this.g;
    const marker = markerOf(this.snap);
    const centre = g?.vertex ?? this.hoverAt;
    const normal = g?.normal ?? this.hoverNormal;
    if (!centre) return { ...EMPTY_VIEW, marker };
    const overlay = [...inferenceLine(this.snap)];
    const r = this.radius(centre);
    const f = alignedFrame(centre, normal);
    const at = (a: number, rr = r) => add(centre, add(mul(f.u, Math.cos(a) * rr), mul(f.v, Math.sin(a) * rr)));
    const an = normal.map(Math.abs);
    const color = an[0] > 1 - 1e-9 ? INFERENCE_STYLES["axis-x"].token : an[1] > 1 - 1e-9 ? INFERENCE_STYLES["axis-y"].token : an[2] > 1 - 1e-9 ? INFERENCE_STYLES["axis-z"].token : "--sel";
    // Half a protractor on the reference's side once it is known, a full ring before.
    const from = g?.ref ? Math.atan2(dot(g.ref, f.v), dot(g.ref, f.u)) : 0;
    const span = g?.ref ? Math.PI : 2 * Math.PI;
    const sideSign = g && g.angle < 0 ? -1 : 1;
    const ring: V3[] = [];
    for (let k = 0; k <= 24; k++) ring.push(at(from + sideSign * (k / 24) * span));
    overlay.push({ kind: g?.ref ? ("strip" as const) : ("loop" as const), points: g?.ref ? ring : ring.slice(0, 24), color, width: 1.5 });
    const ticks: V3[] = [];
    const n = g?.ref ? 13 : 24;
    for (let k = 0; k < n; k++) {
      const t = from + sideSign * k * TICK_DEG * DEG;
      ticks.push(at(t), at(t, r * (k % 3 === 0 ? 0.8 : 0.9)));
    }
    overlay.push({ kind: "segments" as const, points: ticks, color, width: 1 });
    const labels = [];
    if (g?.ref) {
      overlay.push({ kind: "segments" as const, points: [centre, add(centre, mul(g.ref, r))], color: "--sel", width: 1.75 });
      if (Math.abs(g.angle) > 1e-9) {
        const d = rotate(g.ref, g.normal, g.angle);
        // The guide to be: long dashes across the view, like a placed guide.
        const far = Math.max(r * 40, 10);
        overlay.push({ kind: "segments" as const, points: [add(centre, mul(d, -far)), add(centre, mul(d, far))], color: "--inf-guide", width: 1.5, dashed: true });
        const steps = Math.max(2, Math.ceil(Math.abs(g.angle) / (5 * DEG)));
        const arc: V3[] = [];
        for (let k = 0; k <= steps; k++) arc.push(add(centre, mul(rotate(g.ref, g.normal, (g.angle * k) / steps), r * 0.6)));
        overlay.push({ kind: "strip" as const, points: arc, color: "--sel", width: 2 });
        labels.push({ at: add(centre, mul(d, r * 1.15)), text: fmtSlope(g.angle), className: "live" });
      }
    } else if (g?.cursor) {
      overlay.push({ kind: "segments" as const, points: [centre, g.cursor], color: "--sel", width: 1.25, dashed: true });
    }
    return { ...EMPTY_VIEW, marker, overlay, labels };
  }

  /** About 56 px on screen at the vertex (as Rotate's). */
  private radius(c: V3): number {
    const a = this.host.toScreen(c);
    const b = this.host.toScreen(add(c, [0.1, 0, 0]));
    const c2 = this.host.toScreen(add(c, [0, 0.1, 0]));
    const pxPerM = Math.max(Math.hypot(b[0] - a[0], b[1] - a[1]), Math.hypot(c2[0] - a[0], c2[1] - a[1])) / 0.1;
    return pxPerM > 1e-6 ? 56 / pxPerM : 0.1;
  }
}
