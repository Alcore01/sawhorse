// sketchcraft E-31 (H-20261117-8): Push along an axis. The other way to push (Push/Pull
// stays as it was: one face, along its normal). Press on a face of a sketch and move: that
// face, or every selected face when it is one of them, goes up together along blue, with a
// live preview, into one solid (the kernel's extrude_faces; the faces stay). The arrow keys
// switch the axis as for Line (Right red, Left green, Up blue; Down or Shift: the parallel or
// perpendicular line shown), and the axis is a locked inference line: the cursor snaps to
// points of other geometry projected onto it (up to the level of the ridge). Type a distance
// at any point; a negative value goes the other way.
import type { EditResult } from "../kernel/client";
import type { V3 } from "../kernel/types";
import { INFERENCE_STYLES } from "../lib/inferenceStyle";
import { notHere, parseVcb } from "../lib/vcb";
import { sketchEditable } from "../lib/scope";
import { add, dot, mul, sub } from "./draw";
import { LockState, type Lock } from "./lock";
import { EMPTY_VIEW, Tool, markerOf, type InferContext, type Snap, type ToolEvent, type ToolView } from "./tool";

const BLUE: Lock = { dir: [0, 0, 1], kind: "axis-z" };
const AXIS_NAMES: Record<string, string> = { "axis-x": "red", "axis-y": "green", "axis-z": "blue", parallel: "the parallel", perpendicular: "the perpendicular" };
const MIN_PUSH = 1e-5;

interface Gesture {
  faces: [number, number][];
  origin: V3;
  rings: V3[][];
  d: number;
  dragging: boolean;
}

export class PushAxisTool extends Tool {
  readonly id = "pushaxis" as const;
  private snap: Snap | null = null;
  private g: Gesture | null = null;
  private fresh = false;
  private lock = new LockState();
  last: number | null = null;

  /** The axis the faces go along (blue unless an arrow chose another). */
  axis(): Lock {
    return this.lock.lock ?? BLUE;
  }

  valueLabel(): string {
    return "Distance";
  }

  busy(): boolean {
    return this.g !== null;
  }

  reset(): void {
    this.g = null;
    this.fresh = false;
  }

  inferContext(): InferContext {
    const g = this.g;
    if (!g) return {};
    return { start: g.origin, lock: this.axis(), grid: true };
  }

  onKey(e: KeyboardEvent): boolean {
    const start = this.g?.origin ?? [0, 0, 0];
    if (!this.lock.onKey(e.key, this.snap, start)) return false;
    this.host.status(`Along ${AXIS_NAMES[this.axis().kind] ?? this.axis().kind}${this.lock.lock ? "" : " (the default)"}. The arrows switch: Right red, Left green, Up blue.`);
    this.host.changed();
    return true;
  }

  onShift(down: boolean): void {
    if (this.lock.onShift(down, this.snap, this.g?.origin ?? null)) this.host.changed();
  }

  onEvent(e: ToolEvent): void {
    const h = this.host;
    switch (e.type) {
      case "move":
      case "drag": {
        this.snap = e.snap;
        const g = this.g;
        if (g) {
          if (e.type === "drag") g.dragging = true;
          if (e.snap) g.d = dot(sub(e.snap.point, g.origin), this.axis().dir);
          h.showValue(h.fmt(Math.abs(g.d)));
        } else {
          const at = e.picked?.surface?.at;
          h.hover(at?.kind === "region" && sketchEditable(h.state(), at.sketch) ? { kind: "region", sketch: at.sketch, region: at.region } : null);
        }
        h.changed();
        return;
      }
      case "down": {
        if (this.g) {
          this.fresh = false;
          return;
        }
        const s = e.picked?.surface;
        const at = s?.at;
        if (!s || at?.kind !== "region") {
          this.fresh = false;
          if (at?.kind === "face") h.status("Push along an axis takes faces of sketches; a solid's face goes with Push/Pull (P), or Volume > Explode Solid into Faces first.", true);
          return;
        }
        if (!sketchEditable(h.state(), at.sketch)) {
          this.fresh = false;
          return void h.status("That is inside a group or component that is not open: double-click it (Select) to edit inside it.", true);
        }
        // The pressed face, with the other selected faces when it is one of them.
        const sel = h.selection().flatMap((x) => (x.kind === "region" ? [[x.sketch, x.region] as [number, number]] : []));
        const mine = sel.some(([k, r]) => k === at.sketch && r === at.region);
        const faces: [number, number][] = mine ? sel : [[at.sketch, at.region]];
        this.fresh = true;
        this.g = { faces, origin: s.point, rings: [], d: 0, dragging: false };
        const g = this.g;
        void Promise.all(faces.map(([sketch, region]) => h.outline({ kind: "region", sketch, region }))).then((all) => {
          if (this.g === g) {
            g.rings = all.flat();
            h.changed();
          }
        });
        h.status(`Pushing ${faces.length === 1 ? "the face" : `${faces.length} faces together`} along ${AXIS_NAMES[this.axis().kind]}: drag, or move and click, or type a distance. Arrows switch the axis.`);
        h.changed();
        return;
      }
      case "dragend": {
        const g = this.g;
        if (g?.dragging) this.commit(g.d);
        return;
      }
      case "click": {
        const g = this.g;
        if (!g) return;
        if (this.fresh) {
          this.fresh = false;
          return;
        }
        if (Math.abs(g.d) < MIN_PUSH) {
          this.reset();
          h.changed();
          return;
        }
        this.commit(g.d);
        return;
      }
      case "leave":
        h.hover(null);
        return;
      default:
        return;
    }
  }

  onValue(text: string): void {
    const h = this.host;
    const r = parseVcb(text, h.units());
    if (!r.ok) return void h.status(r.error, true);
    if (r.value.kind !== "length") return void h.status(notHere(r.value, "Push along an axis", "a distance (negative goes the other way)"), true);
    const g = this.g;
    if (!g) return void h.status("Press on a face first (Shift+click selects several to push together), then type the distance.", true);
    const sign = g.d < 0 ? -1 : 1;
    this.commit(sign * r.value.m, true);
  }

  private commit(d: number, typed = false): void {
    const g = this.g;
    const axis = this.axis();
    this.reset();
    this.host.changed();
    if (!g || (!typed && Math.abs(d) < MIN_PUSH)) return;
    const h = this.host;
    void h
      .apply({ op: "extrude_faces", faces: g.faces, vector: mul(axis.dir, d) }, (res: EditResult) => {
        const s = res.scene.state.stats;
        return `Pushed ${g.faces.length === 1 ? "the face" : `${g.faces.length} faces`} ${h.fmt(Math.abs(d))} ${h.unitLabel()} along ${AXIS_NAMES[axis.kind] ?? axis.kind} into one solid. ${s.objects} solid(s), ${s.watertight ? "watertight" : "OPEN"}.`;
      })
      .then((ok) => {
        if (ok) {
          this.last = d;
          h.showValue(h.fmt(Math.abs(d)));
        }
      });
  }

  view(): ToolView {
    const g = this.g;
    const marker = markerOf(this.snap);
    if (!g) return { ...EMPTY_VIEW, marker };
    const axis = this.axis();
    const off = mul(axis.dir, g.d);
    const color = g.d < 0 ? "--warn" : "--sel";
    const overlay = [];
    for (const ring of g.rings) {
      overlay.push({ kind: "loop" as const, points: ring.map((p) => add(p, off)), color, width: 2 });
      if (Math.abs(g.d) > MIN_PUSH) overlay.push({ kind: "segments" as const, points: ring.flatMap((p) => [p, add(p, off)]), color, width: 1.25 });
    }
    overlay.push({ kind: "segments" as const, points: [g.origin, add(g.origin, off)], color: INFERENCE_STYLES[axis.kind].token, width: 1.25, dashed: true });
    return { ...EMPTY_VIEW, marker, overlay, labels: [{ at: add(g.origin, off), text: `${g.d < 0 ? "−" : ""}${this.host.fmt(Math.abs(g.d))}`, className: "live" }] };
  }
}
