// sketchcraft E-10: what the selection measures. The area of the selected faces and sketch
// regions (from the triangles the viewport draws, in model units: metres) and the volume of
// the selected solids, groups and component copies (each solid's volume, from the kernel,
// counted once however it was selected). Nothing here asks the kernel: it reads the scene
// the app already has.
import type { DocState, Entity, MeshData } from "../kernel/types";
import { solidsIn } from "./scope";
import { formatVolume, M_PER_IN, type UnitSystem } from "./units";

export interface SelectionMeasure {
  /** Faces and regions selected, and their total area (m²). */
  faces: number;
  area: number;
  /** Solids reached (directly, or inside a selected group or component copy), and their total volume (m³). */
  solids: number;
  volume: number;
}

/** Area (m²) of every mesh part, from its triangles. */
export function partAreas(mesh: MeshData): Float64Array {
  const out = new Float64Array(mesh.parts.length);
  const p = mesh.positions;
  const ix = mesh.indices;
  for (let t = 0; t * 3 + 2 < ix.length; t++) {
    const part = mesh.faceIds[t];
    if (part === undefined || part >= out.length) continue;
    const a = ix[t * 3] * 3;
    const b = ix[t * 3 + 1] * 3;
    const c = ix[t * 3 + 2] * 3;
    const ux = p[b] - p[a], uy = p[b + 1] - p[a + 1], uz = p[b + 2] - p[a + 2];
    const vx = p[c] - p[a], vy = p[c + 1] - p[a + 1], vz = p[c + 2] - p[a + 2];
    const cx = uy * vz - uz * vy, cy = uz * vx - ux * vz, cz = ux * vy - uy * vx;
    out[part] += 0.5 * Math.hypot(cx, cy, cz);
  }
  return out;
}

/** What `selection` measures in this scene. */
export function measureSelection(selection: readonly Entity[], state: DocState | null | undefined, mesh: MeshData | null | undefined): SelectionMeasure {
  const m: SelectionMeasure = { faces: 0, area: 0, solids: 0, volume: 0 };
  let areas: Float64Array | null = null;
  const seenParts = new Set<number>();
  const seenSolids = new Set<number>();
  const volumeOf = new Map((state?.solids ?? []).map((s) => [s.id, s.volume]));
  const addSolid = (id: number) => {
    if (seenSolids.has(id) || !volumeOf.has(id)) return;
    seenSolids.add(id);
    m.solids++;
    m.volume += volumeOf.get(id)!;
  };
  for (const e of selection) {
    if (e.kind === "face" || e.kind === "region") {
      if (!mesh) continue;
      const i = mesh.parts.findIndex((p) => (e.kind === "face" ? p.kind === "face" && p.owner === e.object && p.index === e.face : p.kind === "region" && p.owner === e.sketch && p.index === e.region));
      if (i < 0 || seenParts.has(i)) continue;
      seenParts.add(i);
      areas ??= partAreas(mesh);
      m.faces++;
      m.area += areas[i];
    } else if (e.kind === "object") {
      addSolid(e.object);
    } else if (e.kind === "node" && state?.graph) {
      for (const s of solidsIn(state.graph, e.node)) addSolid(s);
    }
  }
  return m;
}

/** An area in the display units: m² (metric) or sq ft (imperial), two decimals. */
export function formatSurface(m2: number, system: UnitSystem): string {
  const metric = system === "mm" || system === "cm" || system === "m";
  const v = metric ? m2 : m2 / (12 * M_PER_IN) ** 2;
  const s = v.toFixed(2);
  return `${s === "-0.00" ? "0.00" : s} ${metric ? "m²" : "sq ft"}`;
}

/** The selection's measures for the viewport's badge: "3 faces · 52.30 m²", "1 solid · 466.18 m³". */
export function describeMeasure(m: SelectionMeasure, system: UnitSystem, closed: number | null = null): string {
  const parts: string[] = [];
  if (m.faces) parts.push(`${m.faces} face${m.faces === 1 ? "" : "s"} · ${formatSurface(m.area, system)}`);
  // sketchcraft E-36: loose faces that close a volume tell it.
  if (closed !== null) parts.push(`enclosing ${formatVolume(closed, system)}`);
  if (m.solids) parts.push(`${m.solids} solid${m.solids === 1 ? "" : "s"} · ${formatVolume(m.volume, system)}`);
  return parts.join(" · ");
}
