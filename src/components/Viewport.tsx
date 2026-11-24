import { t, tChip } from "../lib/i18n";
import { useEffect, useRef, useState, type ReactNode } from "react";
import type { MeshData, MeshPart } from "../kernel/types";
import { virtualShift } from "./KeyBar";
import type { Vec3 } from "../lib/edit";
import { INFERENCE_STYLES, LEGEND_KINDS, SNAP_SWITCHES, type InferenceKind } from "../lib/inferenceStyle";
import { isMacControlClick } from "../lib/platform";
import type { UnitSystem } from "../lib/units";
import { NO_HIGHLIGHT, ViewportEngine, webgl2Available, type Highlight, type OverlayItem, type Projection, type ViewId } from "../viewport/engine";
import { gizmoDots } from "../lib/axisGizmo";

export type KernelState = "loading" | "ready" | "missing" | "error";

/** A pointer event on the canvas, in canvas CSS pixels, with the modifier keys. */
export interface ViewportPointer {
  type: "down" | "move" | "up" | "dblclick" | "leave";
  x: number;
  y: number;
  /** 0 left, 1 middle, 2 right. */
  button: number;
  shift: boolean;
  ctrl: boolean;
  alt: boolean;
  /** A Control+click on macOS, the Mac's right click (src/lib/platform.ts): it pans, and the
   * tool never sees its press. `ctrl` is true for Ctrl or Cmd. */
  macControl?: boolean;
}

/** The inference marker: a dot in the inference's colour plus its tooltip. */
export interface Marker {
  at: Vec3;
  kind: InferenceKind;
  tip: string;
  /** Lane 23: the centre of the circle, arc or polygon under the cursor (a small dot labelled
   * "Centre"; moving there snaps to it). */
  centre?: Vec3 | null;
}

/** A text label pinned to a world point (dimension text, tape readout). */
export interface Label {
  at: Vec3;
  text: string;
  className?: string;
  /** Hide the label while a face is between the eye and `at` (stored dimensions, E-46). */
  occlude?: boolean;
}

/** Window (left to right, solid) or crossing (right to left, dashed) selection rectangle. */
export interface SelectionBox {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
  crossing: boolean;
}

export interface ViewportProps {
  mesh: MeshData | null;
  units: UnitSystem;
  /** Requested standard view; `viewSeq` changes re-apply it even when the id repeats. */
  view: ViewId;
  viewSeq: number;
  projection: Projection;
  /** Bumped to request a zoom extents. */
  fitSeq: number;
  /** HUD chips, top-left, as in the mock. */
  hud: { text: string; tone?: "ok" | "err" }[];
  kernel: KernelState;
  /** Data attributes for tests and captures (state hash, watertight, objects, undo). */
  info?: { hash?: string; watertight?: boolean; objects?: number; canUndo?: boolean; canRedo?: boolean; volume?: number; solids?: number };
  highlight?: Highlight;
  overlay?: OverlayItem[];
  marker?: Marker | null;
  labels?: Label[];
  box?: SelectionBox | null;
  cursor?: string;
  /** Number of selected entities (a data attribute for tests). */
  selectionCount?: number;
  /** Kind of the entity under the cursor (a data attribute for tests). */
  hoverKind?: string;
  onPointer?: (p: ViewportPointer) => void;
  /** The camera left the standard view (orbit) or changed projection. */
  onCamera?: (view: ViewId | null, projection: Projection) => void;
  /** The engine, once it exists (null when it goes away). */
  onEngine?: (engine: ViewportEngine | null) => void;
  /** sketchcraft E-16: the legend as snapping switches (absent: the plain legend). */
  snaps?: { on: (id: string) => boolean; toggle: (id: string) => void };
  /** sketchcraft E-51: controls shown first in the bar along the bottom of the view (the
   * selection filter, the Point panel), before the snapping switches. */
  bottomLead?: ReactNode;
  /** sketchcraft E-55: a click on the axes in the corner: look from that side. */
  onAxisView?: (view: ViewId) => void;
  /** sketchcraft E-57: a right click that did not drag (a drag pans): the menu at the pointer
   * (page coordinates). */
  onRightClick?: (x: number, y: number) => void;
  /** sketchcraft E-62: where north is (degrees clockwise from green), for the compass rose;
   * a click on the rose asks for it. */
  north?: number;
  onNorth?: () => void;
}

/** Read-only hooks for e2e tests and lane-end captures (scripts/capture*.mjs, e2e/*.spec.ts). */
export interface SawhorseHooks {
  visibleEndpoints(): [number, number][];
  parts(): MeshPart[];
  partScreenCenter(part: number): [number, number] | null;
  worldToScreen(p: Vec3): [number, number];
  camera(): ReturnType<ViewportEngine["cameraSummary"]>;
  grid(): ReturnType<ViewportEngine["gridState"]>;
  /** Coarse pick under canvas pixel (x, y): world point and part index (-1 = ground). */
  pick(x: number, y: number): { point: Vec3; part: number } | null;
  /** Read back canvas pixels (r, g, b) at canvas pixel (x, y). */
  pixel(x: number, y: number): [number, number, number];
  /** Milliseconds per frame over `n` renders with the camera turning (lane 10). */
  frameTime(n?: number): number;
  /** Triangles of the mesh the engine draws (what the render passes read). */
  engineTriangles(): number;
  /** Lane 15: edge segments drawn as lines, soft, hidden; whether smooth normals came. */
  edgeCounts(): ReturnType<ViewportEngine["edgeCounts"]>;
  /** Lane 15: the grid's line colours (they follow the style's background). */
  gridColours(): ReturnType<ViewportEngine["gridColours"]>;
  /** Lane 16b: what the highlight draws (fat segments; solids and parts drawn from the state textures). */
  highlightCounts(): ReturnType<ViewportEngine["highlightCounts"]>;
  /** Lane 21: the point navigation holds under canvas pixel (x, y), or null: the orbit pivot
   * (`snap`, the default: a vertex or edge a few pixels off counts), or what a pan drags. */
  navPick(x: number, y: number, snap?: boolean): Vec3 | null;
  /** Lane 21: the gesture under way and the orbit pivot (null when not orbiting). */
  nav(): ReturnType<ViewportEngine["navState"]>;
}

export function Viewport(props: ViewportProps) {
  const { mesh, units, view, viewSeq, projection, fitSeq, hud, kernel, info, highlight, overlay, marker, labels, box, cursor, selectionCount, hoverKind, onPointer, onCamera, onEngine } = props;
  // sketchcraft E-57: where the right button went down (a right click opens the menu).
  const rightDown = useRef<{ x: number; y: number } | null>(null);
  const hostRef = useRef<HTMLElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const engineRef = useRef<ViewportEngine | null>(null);
  const meshRef = useRef<MeshData | null>(mesh);
  meshRef.current = mesh;
  const [glError, setGlError] = useState<string | null>(null);
  const [frame, setFrame] = useState(0);
  const cb = useRef({ onEngine, onCamera, onPointer });
  cb.current = { onEngine, onCamera, onPointer };
  const [camAttr, setCamAttr] = useState<{ view: ViewId | null; projection: Projection; eye: string }>({ view: "iso", projection: "parallel", eye: "" });

  // Engine lifetime.
  useEffect(() => {
    const canvas = canvasRef.current;
    const host = hostRef.current;
    if (!canvas || !host) return;
    if (!webgl2Available()) {
      setGlError("WebGL2 is not available in this browser.");
      return;
    }
    let engine: ViewportEngine;
    try {
      engine = new ViewportEngine(canvas);
    } catch (e) {
      setGlError(e instanceof Error ? e.message : String(e));
      return;
    }
    engineRef.current = engine;
    cb.current.onEngine?.(engine);
    const hooks: SawhorseHooks = {
      visibleEndpoints: () => engine.visibleEndpoints(),
      parts: () => meshRef.current?.parts ?? [],
      partScreenCenter: (part) => engine.partScreenCenter(part),
      worldToScreen: (p) => engine.worldToScreen(p),
      camera: () => engine.cameraSummary(),
      grid: () => engine.gridState(),
      pick: (x, y) => engine.pick(x, y),
      pixel: (x, y) => engine.readPixel(x, y),
      frameTime: (n) => engine.benchmark(n),
      engineTriangles: () => (engine.captureSource().mesh?.indices.length ?? 0) / 3,
      edgeCounts: () => engine.edgeCounts(),
      gridColours: () => engine.gridColours(),
      highlightCounts: () => engine.highlightCounts(),
      navPick: (x, y, snap) => engine.navPick(x, y, snap ?? true),
      nav: () => engine.navState(),
    };
    const w = window as unknown as { __sawhorse?: Record<string, unknown> };
    w.__sawhorse = { ...(w.__sawhorse ?? {}), ...hooks };
    const ro = new ResizeObserver(() => {
      const r = host.getBoundingClientRect();
      engine.resize(r.width, r.height);
    });
    ro.observe(host);
    const r = host.getBoundingClientRect();
    engine.resize(r.width, r.height);
    let last = "";
    const off = engine.onRender(() => {
      // Labels and the marker follow the camera; the camera summary feeds tests.
      setFrame((f) => (f + 1) % 1_000_000);
      const s = engine.cameraSummary();
      const key = `${s.view}|${s.projection}|${s.eye.join(",")}|${s.zoom}`;
      if (key !== last) {
        last = key;
        setCamAttr({ view: s.view, projection: s.projection, eye: `${s.eye.join(",")}@${s.zoom}` });
        cb.current.onCamera?.(s.view, s.projection);
      }
    });

    // Theme: OS preference and the data-theme override both repaint the scene.
    const mq = window.matchMedia?.("(prefers-color-scheme: dark)");
    const onTheme = () => engine.applyTheme();
    mq?.addEventListener?.("change", onTheme);
    const mo = new MutationObserver(onTheme);
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });

    return () => {
      off();
      ro.disconnect();
      mo.disconnect();
      mq?.removeEventListener?.("change", onTheme);
      cb.current.onEngine?.(null);
      engine.dispose();
      engineRef.current = null;
    };
  }, []);

  useEffect(() => {
    engineRef.current?.setMesh(mesh);
  }, [mesh]);

  useEffect(() => {
    engineRef.current?.setUnits(units);
  }, [units]);

  useEffect(() => {
    engineRef.current?.setView(view);
  }, [view, viewSeq]);

  useEffect(() => {
    engineRef.current?.setProjection(projection);
  }, [projection]);

  useEffect(() => {
    if (fitSeq > 0) engineRef.current?.fit();
  }, [fitSeq]);

  useEffect(() => {
    engineRef.current?.setHighlight(highlight ?? NO_HIGHLIGHT);
  }, [highlight, mesh]);

  useEffect(() => {
    engineRef.current?.setOverlay(overlay ?? []);
  }, [overlay]);

  const local = (e: React.PointerEvent<HTMLCanvasElement> | React.MouseEvent<HTMLCanvasElement>): [number, number] => {
    const rect = e.currentTarget.getBoundingClientRect();
    return [e.clientX - rect.left, e.clientY - rect.top];
  };
  const send = (type: ViewportPointer["type"], e: React.PointerEvent<HTMLCanvasElement> | React.MouseEvent<HTMLCanvasElement>) => {
    const [x, y] = local(e);
    cb.current.onPointer?.({ type, x, y, button: e.button, shift: e.shiftKey || virtualShift(), ctrl: e.ctrlKey || e.metaKey, alt: e.altKey, macControl: isMacControlClick(e) });
  };

  const tri = mesh ? mesh.indices.length / 3 : 0;
  const engine = engineRef.current;
  void frame; // re-render with the camera so pinned labels follow it
  const markerXY = marker && engine ? engine.worldToScreen(marker.at) : null;
  // Lane 23: the centre of the curve under the cursor, when the marker is not already on it.
  const centre = marker?.centre && marker.kind !== "centre" ? marker.centre : null;
  const centreXY = centre && engine ? engine.worldToScreen(centre) : null;
  const style = marker ? INFERENCE_STYLES[marker.kind] : null;
  // Lane 21: the orbit pivot's marker, while the middle button (or one finger, or lane 30's
  // left button under the Orbit tool or with Alt) is down.
  const nav = engine?.navState() ?? null;
  const pivotXY = nav?.pivot && engine ? engine.worldToScreen(nav.pivot) : null;

  return (
    <main
      className="vp"
      aria-label="Viewport"
      ref={hostRef}
      data-kernel={kernel}
      data-triangles={tri}
      data-parts={mesh?.parts.length ?? 0}
      data-watertight={info?.watertight === undefined ? undefined : String(info.watertight)}
      data-hash={info?.hash}
      data-objects={info?.objects}
      data-volume={info?.volume === undefined ? undefined : String(info.volume)}
      data-can-undo={info?.canUndo === undefined ? undefined : String(info.canUndo)}
      data-can-redo={info?.canRedo === undefined ? undefined : String(info.canRedo)}
      data-view={camAttr.view ?? "custom"}
      data-projection={camAttr.projection}
      data-camera={camAttr.eye}
      data-inference={marker?.kind ?? ""}
      data-centre={centre ? centre.map((x) => x.toFixed(6)).join(",") : ""}
      data-selection={selectionCount ?? 0}
      data-hover={hoverKind ?? ""}
      data-nav={nav?.mode ?? ""}
    >
      <canvas
        id="cv"
        ref={canvasRef}
        aria-label="3D viewport"
        style={cursor ? { cursor } : undefined}
        onPointerMove={(e) => send("move", e)}
        onPointerDown={(e) => {
          if (e.button === 1) e.preventDefault(); // no autoscroll on middle press
          if (e.button === 2) rightDown.current = { x: e.clientX, y: e.clientY };
          send("down", e);
        }}
        onPointerUp={(e) => {
          const r = rightDown.current;
          if (e.button === 2 && r) {
            rightDown.current = null;
            if (Math.hypot(e.clientX - r.x, e.clientY - r.y) < 5) props.onRightClick?.(e.clientX, e.clientY);
          }
          send("up", e);
        }}
        onDoubleClick={(e) => send("dblclick", e)}
        onPointerLeave={(e) => send("leave", e)}
        onContextMenu={(e) => e.preventDefault()}
      ></canvas>
      <div className="hud">
        {glError ? <span className="err">{glError}</span> : null}
        {hud.map((h, i) => (
          <span key={i} className={h.tone}>
            {tChip(h.text)}
          </span>
        ))}
      </div>
      {engine
        ? (labels ?? []).map((l, i) => {
            if (l.occlude && engine.occluded(l.at)) return null;
            const [x, y] = engine.worldToScreen(l.at);
            return (
              <div key={i} className={`vlabel ${l.className ?? ""}`} style={{ left: x, top: y }}>
                {l.text}
              </div>
            );
          })
        : null}
      {box ? (
        <div
          className={box.crossing ? "selbox crossing" : "selbox"}
          style={{ left: Math.min(box.x0, box.x1), top: Math.min(box.y0, box.y1), width: Math.abs(box.x1 - box.x0), height: Math.abs(box.y1 - box.y0) }}
        ></div>
      ) : null}
      {pivotXY ? <span className="orbit-pivot" data-testid="orbit-pivot" aria-hidden="true" style={{ left: pivotXY[0], top: pivotXY[1] }}></span> : null}
      {centreXY ? (
        <>
          <span className="snap centre-hint" aria-hidden="true" style={{ left: centreXY[0], top: centreXY[1], background: `var(${INFERENCE_STYLES.centre.token})` }}></span>
          <div className="tip centre-tip" data-testid="centre-hint" style={{ left: centreXY[0] + 10, top: centreXY[1] - 22 }}>
            <i style={{ background: `var(${INFERENCE_STYLES.centre.token})` }}></i>
            {INFERENCE_STYLES.centre.label}
          </div>
        </>
      ) : null}
      {marker && markerXY && style ? (
        <>
          <span className="snap" aria-hidden="true" style={{ left: markerXY[0], top: markerXY[1], background: `var(${style.token})` }}></span>
          <div className="tip" id="tip" style={{ left: markerXY[0] + 12, top: markerXY[1] - 26 }}>
            <i style={{ background: `var(${style.token})` }}></i>
            {t(marker.tip)}
          </div>
        </>
      ) : null}
      {props.snaps ? (
        // sketchcraft E-16: the legend is the snapping switches (click to turn one off).
        // E-51: in a bar along the bottom, after the selection filter, each choice boxed.
        <div className="bottom-bar">
          {props.bottomLead}
        <div className="legend snaps" role="group" aria-label="Snapping">
          <b className="bar-title">{t("Snap")}</b>
          {SNAP_SWITCHES.map((s) => {
            const on = props.snaps!.on(s.id);
            return (
              <button key={s.id} type="button" aria-pressed={on} className={on ? "on" : "off"} title={`${t(s.label)}: ${on ? t("on") : t("off")} (${t("click to switch")})`} onClick={() => props.snaps!.toggle(s.id)}>
                <i style={{ background: on ? `var(${s.token})` : "transparent", borderColor: `var(${s.token})` }}></i>
                {t(s.label)}
              </button>
            );
          })}
          <span>
            <i style={{ background: `var(${INFERENCE_STYLES["on-face"].token})` }}></i>
            {t(INFERENCE_STYLES["on-face"].label)}
          </span>
        </div>
        </div>
      ) : (
        <div className="legend">
          {LEGEND_KINDS.map((k) => INFERENCE_STYLES[k]).map((s) => (
            <span key={s.token}>
              <i style={{ background: `var(${s.token})` }}></i>
              {t(s.label)}
            </span>
          ))}
        </div>
      )}
      <div className="axes">{t("x ─ red · y ─ green · z ─ blue")}</div>
      {engine ? <AxisGizmo frame={engine.axisFrame()} onView={props.onAxisView} /> : null}
      {engine ? <CompassRose frame={engine.axisFrame()} north={props.north ?? 0} onClick={props.onNorth} /> : null}
    </main>
  );
}

/** sketchcraft E-55: the axes in the top right corner, turning with the camera; a click on a
 * dot looks from that side (X: Right, -X: Left, Y: Back, -Y: Front, Z: Top, -Z: Bottom). */
function AxisGizmo({ frame, onView }: { frame: { x: Vec3; y: Vec3; z: Vec3 }; onView?: (view: ViewId) => void }) {
  const size = 96;
  const c = size / 2;
  const r = 32;
  const dots = gizmoDots(frame, r);
  const names: Record<string, string> = { right: "Right", left: "Left", back: "Back", front: "Front", top: "Top", bottom: "Bottom" };
  return (
    <svg className="axis-gizmo" data-testid="axis-gizmo" width={size} height={size} viewBox={`0 0 ${size} ${size}`} role="group" aria-label="Axes">
      <circle cx={c} cy={c} r={c - 2} className="axis-gizmo-bg" />
      {dots
        .filter((d) => d.positive)
        .map((d) => (
          <line key={`l${d.id}`} x1={c} y1={c} x2={c + d.x} y2={c + d.y} stroke={d.color} strokeWidth={2.5} strokeLinecap="round" />
        ))}
      {dots.map((d) => (
        <g key={d.id} className="axis-dot" role="button" aria-label={`${d.positive ? "" : "minus "}${d.id.replace("-", "").toUpperCase()}: ${names[d.view]} view`} onClick={() => onView?.(d.view)}>
          <title>{`${d.positive ? "+" : "−"}${d.id.replace("-", "").toUpperCase()}: ${t("look from this side")} (${t(names[d.view])})`}</title>
          <circle cx={c + d.x} cy={c + d.y} r={d.positive ? 9 : 7} fill={d.positive ? d.color : "var(--panel)"} fillOpacity={d.positive ? 1 : 0.85} stroke={d.color} strokeWidth={2} />
          {d.label ? (
            <text x={c + d.x} y={c + d.y} dy="0.35em" textAnchor="middle" className="axis-letter">
              {d.label}
            </text>
          ) : null}
        </g>
      ))}
    </svg>
  );
}

/** sketchcraft E-62: the compass rose under the axes, lying on the ground as the camera sees it
 * (foreshortened when the view is tilted), with north where the document says (turned with the
 * building by the Align commands, or set by View > North…). A click asks where north is. */
function CompassRose({ frame, north, onClick }: { frame: { x: Vec3; y: Vec3; z: Vec3 }; north: number; onClick?: () => void }) {
  const size = 96;
  const c = size / 2;
  const r = 34;
  const rad = (north * Math.PI) / 180;
  // A ground direction (dx, dy) as the camera sees it, in pixels (y down).
  const at = (dx: number, dy: number, k = r): [number, number] => {
    const sx = dx * frame.x[0] + dy * frame.y[0];
    const sy = dx * frame.x[1] + dy * frame.y[1];
    return [c + sx * k, c - sy * k];
  };
  const n: [number, number] = [Math.sin(rad), Math.cos(rad)];
  const e: [number, number] = [Math.cos(rad), -Math.sin(rad)];
  const p = (u: [number, number], k = r) => at(u[0], u[1], k);
  const neg = (u: [number, number]): [number, number] => [-u[0], -u[1]];
  const side = (u: [number, number], k: number): [number, number] => [u[0] * k, u[1] * k];
  const tip = p(n);
  const tail = p(neg(n), r * 0.75);
  const l = p(side(neg(e), 0.16), r);
  const rr = p(side(e, 0.16), r);
  const pts = (...q: [number, number][]) => q.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(" ");
  const label = (u: [number, number], s: string) => {
    const [x, y] = p(u, r + 9);
    return (
      <text x={x} y={y} dy="0.35em" textAnchor="middle" className={s === "N" ? "rose-n" : "rose-l"}>
        {t(s)}
      </text>
    );
  };
  return (
    <svg className="compass-rose" data-testid="compass-rose" width={size} height={size} viewBox={`0 0 ${size} ${size}`} role="button" aria-label={`${t("North")}: ${north.toFixed(1)}°`} onClick={onClick}>
      <title>{`${t("North")}: ${north.toFixed(1)}° ${t("from green")} (${t("click to set it")})`}</title>
      <line x1={p(e)[0]} y1={p(e)[1]} x2={p(neg(e))[0]} y2={p(neg(e))[1]} className="rose-axis" />
      <polygon points={pts(tip, [c, c], l)} className="rose-north" />
      <polygon points={pts(tip, [c, c], rr)} className="rose-north dark" />
      <polygon points={pts(tail, [c, c], l, rr)} className="rose-south" />
      {label(n, "N")}
      {label(e, "E")}
      {label(neg(n), "S")}
      {label(neg(e), "W")}
    </svg>
  );
}
