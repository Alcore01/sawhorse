// sketchcraft E-56: the Info pane (JC, 20 nov. 2026 : « c'est surtout les plus de l'interface
// qui m'intéresse », then « Panneau Propriétés » ; « on reprend l'idée pas le code »): what
// the selection measures, for a PEB survey: a face's area, kind (wall, roof, floor) and
// orientation or slope; a solid's volume and its envelope split into walls, roofs and floor;
// an edge's length; a point's coordinates. Worked out from the view mesh (single precision:
// plenty for areas and volumes to the square centimetre).
import type { DocState, Entity, MeshData, V3 } from "../kernel/types";
import type { UnitSystem } from "./units";
import { formatLength } from "./units";
import { partsOf } from "./entities";

export interface InfoRow {
  label: string;
  value: string;
}

export interface InfoSheet {
  title: string;
  rows: InfoRow[];
  note?: string;
}

const IMPERIAL = (u: UnitSystem) => u === "in" || u === "frac-in" || u === "ft-in";
const FT = 0.3048;

export function fmtArea(m2: number, u: UnitSystem): string {
  return IMPERIAL(u) ? `${(m2 / FT ** 2).toFixed(2)} ft²` : `${m2.toFixed(2)} m²`;
}

export function fmtVolume(m3: number, u: UnitSystem): string {
  return IMPERIAL(u) ? `${(m3 / FT ** 3).toFixed(2)} ft³` : `${m3.toFixed(3)} m³`;
}

const len = (u: UnitSystem) => (m: number) => formatLength(m, u).replace(/^~\s*/, "");

/** A face's kind from its outward normal: a wall within 10° of upright, a floor facing down,
 * a roof otherwise (a flat roof too). */
export function faceKind(n: V3): "wall" | "roof" | "floor" {
  const l = Math.hypot(n[0], n[1], n[2]) || 1;
  const z = n[2] / l;
  if (Math.abs(z) <= Math.sin((10 * Math.PI) / 180)) return "wall";
  return z < 0 ? "floor" : "roof";
}

const POINTS = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];

/** Where a face looks, in degrees clockwise from north (+Y, green, as the sun uses), and the
 * nearest of the eight compass points. */
export function azimuth(n: V3): { deg: number; point: string } {
  let deg = (Math.atan2(n[0], n[1]) * 180) / Math.PI;
  if (deg < 0) deg += 360;
  return { deg, point: POINTS[Math.round(deg / 45) % 8] };
}

/** The slope of a face from level, degrees (0 flat, 90 upright). */
export function slope(n: V3): number {
  const l = Math.hypot(n[0], n[1], n[2]) || 1;
  return (Math.acos(Math.min(1, Math.abs(n[2]) / l)) * 180) / Math.PI;
}

/** Area of each pickable part of the mesh, square metres (index as `mesh.parts`). */
export function partAreas(mesh: MeshData): number[] {
  const out = new Array<number>(mesh.parts.length).fill(0);
  const p = mesh.positions;
  for (let t = 0; t < mesh.faceIds.length; t++) {
    const [a, b, c] = [mesh.indices[3 * t], mesh.indices[3 * t + 1], mesh.indices[3 * t + 2]];
    const u = [p[3 * b] - p[3 * a], p[3 * b + 1] - p[3 * a + 1], p[3 * b + 2] - p[3 * a + 2]];
    const v = [p[3 * c] - p[3 * a], p[3 * c + 1] - p[3 * a + 1], p[3 * c + 2] - p[3 * a + 2]];
    const x = u[1] * v[2] - u[2] * v[1];
    const y = u[2] * v[0] - u[0] * v[2];
    const z = u[0] * v[1] - u[1] * v[0];
    out[mesh.faceIds[t]] += Math.hypot(x, y, z) / 2;
  }
  return out;
}

/** A solid's volume from its triangles (the divergence theorem), cubic metres. */
export function objectVolume(mesh: MeshData, object: number): number {
  const p = mesh.positions;
  let v = 0;
  for (let t = 0; t < mesh.faceIds.length; t++) {
    const part = mesh.parts[mesh.faceIds[t]];
    if (part?.kind !== "face" || part.owner !== object) continue;
    const [a, b, c] = [mesh.indices[3 * t], mesh.indices[3 * t + 1], mesh.indices[3 * t + 2]];
    const ax = p[3 * a], ay = p[3 * a + 1], az = p[3 * a + 2];
    const bx = p[3 * b], by = p[3 * b + 1], bz = p[3 * b + 2];
    const cx = p[3 * c], cy = p[3 * c + 1], cz = p[3 * c + 2];
    v += ax * (by * cz - bz * cy) - ay * (bx * cz - bz * cx) + az * (bx * cy - by * cx);
  }
  return Math.abs(v / 6);
}

interface FaceFacts {
  area: number;
  kind: "wall" | "roof" | "floor";
  normal: V3;
}

function facesOf(mesh: MeshData, areas: number[], pick: (i: number) => boolean): FaceFacts[] {
  const out: FaceFacts[] = [];
  mesh.parts.forEach((p, i) => {
    if (pick(i)) out.push({ area: areas[i], kind: faceKind(p.normal), normal: p.normal });
  });
  return out;
}

/** The envelope of some faces, split by kind. */
function envelopeRows(faces: FaceFacts[], u: UnitSystem): InfoRow[] {
  const sum = (k?: FaceFacts["kind"]) => faces.filter((f) => !k || f.kind === k).reduce((a, f) => a + f.area, 0);
  return [
    { label: "Envelope", value: fmtArea(sum(), u) },
    { label: "Walls", value: fmtArea(sum("wall"), u) },
    { label: "Roofs", value: fmtArea(sum("roof"), u) },
    { label: "Floor", value: fmtArea(sum("floor"), u) },
  ];
}

function faceRows(f: FaceFacts, u: UnitSystem): InfoRow[] {
  const rows: InfoRow[] = [
    { label: "Area", value: fmtArea(f.area, u) },
    { label: "Kind", value: f.kind === "wall" ? "Wall" : f.kind === "roof" ? "Roof" : "Floor (facing down)" },
  ];
  if (f.kind !== "floor" && Math.hypot(f.normal[0], f.normal[1]) > 1e-6) {
    const a = azimuth(f.normal);
    rows.push({ label: "Faces", value: `${a.point} · ${a.deg.toFixed(1)}° from north` });
  }
  if (f.kind === "roof") rows.push({ label: "Slope", value: `${slope(f.normal).toFixed(1)}°` });
  return rows;
}

const pt = (p: V3, u: UnitSystem): InfoRow[] => {
  const f = len(u);
  return [
    { label: "X", value: f(p[0]) },
    { label: "Y", value: f(p[1]) },
    { label: "Z", value: f(p[2]) },
  ];
};

/** What the Info pane shows for `selection` (the whole model when nothing is selected). */
export function selectionInfo(selection: Entity[], mesh: MeshData | null, state: DocState | null, u: UnitSystem): InfoSheet {
  if (!mesh) return { title: "Nothing to measure", rows: [] };
  const areas = partAreas(mesh);
  const partOf = (e: Entity) => mesh.parts.findIndex((p) => (e.kind === "face" ? p.kind === "face" && p.owner === e.object && p.index === e.face : e.kind === "region" ? p.kind === "region" && p.owner === e.sketch && p.index === e.region : false));
  const sel = selection;
  if (!sel.length) {
    const objects = [...new Set(mesh.parts.filter((p) => p.kind === "face").map((p) => p.owner))];
    const vol = objects.reduce((a, o) => a + objectVolume(mesh, o), 0);
    return {
      title: "Model",
      rows: [{ label: "Solids", value: String(objects.length) }, { label: "Volume", value: fmtVolume(vol, u) }, ...envelopeRows(facesOf(mesh, areas, (i) => mesh.parts[i].kind === "face"), u)],
      note: state && !state.stats.watertight ? "Some solid is not closed: its volume is not counted." : "Select a face, an edge, a point or a solid to measure it.",
    };
  }
  if (sel.length === 1) {
    const e = sel[0];
    switch (e.kind) {
      case "face":
      case "region": {
        const i = partOf(e);
        if (i < 0) break;
        const f: FaceFacts = { area: areas[i], kind: faceKind(mesh.parts[i].normal), normal: mesh.parts[i].normal };
        return { title: e.kind === "face" ? "Face of a solid" : "Face", rows: faceRows(f, u) };
      }
      case "object": {
        return {
          title: "Solid",
          rows: [{ label: "Volume", value: fmtVolume(objectVolume(mesh, e.object), u) }, ...envelopeRows(facesOf(mesh, areas, (i) => mesh.parts[i].kind === "face" && mesh.parts[i].owner === e.object), u)],
        };
      }
      case "edge": {
        const d: V3 = [e.b[0] - e.a[0], e.b[1] - e.a[1], e.b[2] - e.a[2]];
        const l = Math.hypot(...d);
        const rows: InfoRow[] = [{ label: "Length", value: len(u)(l) }];
        const h = Math.hypot(d[0], d[1]);
        rows.push({ label: "Slope", value: `${((Math.atan2(Math.abs(d[2]), h) * 180) / Math.PI).toFixed(1)}°` });
        return { title: "Edge", rows };
      }
      case "vertex":
        return { title: "Point", rows: pt(e.at, u), note: "Edit X, Y, Z in the Point box at the bottom of the view." };
      case "node": {
        // A group or component: the solids inside it.
        const parts = new Set(partsOf(e, mesh, state?.graph));
        const objects = [...new Set([...parts].flatMap((i) => (mesh.parts[i]?.kind === "face" ? [mesh.parts[i].owner] : [])))];
        return {
          title: "Group",
          rows: [{ label: "Solids", value: String(objects.length) }, { label: "Volume", value: fmtVolume(objects.reduce((a, o) => a + objectVolume(mesh, o), 0), u) }, ...envelopeRows(facesOf(mesh, areas, (i) => parts.has(i) && mesh.parts[i].kind === "face"), u)],
        };
      }
      default:
        break;
    }
  }
  // Several things: faces add up; solids add up.
  const faceIdx = sel.flatMap((e) => (e.kind === "face" || e.kind === "region" ? [partOf(e)] : [])).filter((i) => i >= 0);
  const objects = [...new Set(sel.flatMap((e) => (e.kind === "object" ? [e.object] : [])))];
  const rows: InfoRow[] = [];
  if (faceIdx.length) rows.push({ label: "Faces", value: String(faceIdx.length) }, ...envelopeRows(facesOf(mesh, areas, (i) => faceIdx.includes(i)), u).map((r) => (r.label === "Envelope" ? { label: "Total area", value: r.value } : r)));
  if (objects.length) rows.push({ label: "Solids", value: String(objects.length) }, { label: "Volume", value: fmtVolume(objects.reduce((a, o) => a + objectVolume(mesh, o), 0), u) });
  const edges = sel.filter((e) => e.kind === "edge");
  if (edges.length) rows.push({ label: "Edges", value: String(edges.length) }, { label: "Total length", value: len(u)(edges.reduce((a, e) => (e.kind === "edge" ? a + Math.hypot(e.b[0] - e.a[0], e.b[1] - e.a[1], e.b[2] - e.a[2]) : a), 0)) });
  const points = sel.filter((e) => e.kind === "vertex").length;
  if (points) rows.push({ label: "Points", value: String(points) });
  return { title: `${sel.length} selected`, rows };
}
