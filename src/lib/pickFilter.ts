// sketchcraft E-49: the selection filter. JC (19 nov. 2026): « un mode point face et arêtes
// (coordonnées des points) », then « Filtre de sélection ». With Points, Edges or Faces on, a
// click or a box takes only that kind: the nearest point or edge of the solid under the
// cursor (on screen), or the face under it. "all" keeps what the pick gives.
import type { Entity, MeshData, Picked, V3 } from "../kernel/types";
import { type Box2, type P2, inBox, norm, segmentTouches, triangleTouches } from "./entities";

export type PickFilter = "all" | "vertex" | "edge" | "face";

/** How near (screen pixels) the cursor must be to a point or an edge to take it. */
export const FILTER_PX = 30;

type Owner = { kind: "object" | "sketch"; id: number };

function ownerOf(e: Entity, surface: Picked["surface"]): Owner | null {
  switch (e.kind) {
    case "vertex":
    case "edge":
    case "curve":
      return e.owner.kind === "object" || e.owner.kind === "sketch" ? { kind: e.owner.kind, id: e.owner.id } : null;
    case "face":
    case "object":
      return { kind: "object", id: e.object };
    case "region":
      return { kind: "sketch", id: e.sketch };
    default:
      if (surface?.at.kind === "face") return { kind: "object", id: surface.at.object };
      if (surface?.at.kind === "region") return { kind: "sketch", id: surface.at.sketch };
      return null;
  }
}

/** The edge segments (index into mesh.edges) of an owner. */
function edgesOf(mesh: MeshData, owner: Owner): number[] {
  const out: number[] = [];
  for (let i = 0; i < mesh.edgeOwner.length; i++) {
    const o = mesh.edgeOwners[mesh.edgeOwner[i]];
    if (o && o.kind === owner.kind && o.id === owner.id) out.push(i);
  }
  return out;
}

const end = (e: Float32Array, i: number, k: 0 | 1): V3 => [e[6 * i + 3 * k], e[6 * i + 3 * k + 1], e[6 * i + 3 * k + 2]];

function edgeEntity(mesh: MeshData, owner: Owner, i: number): Entity {
  const flags = mesh.edgeFlags?.[i];
  return { kind: "edge", owner, a: end(mesh.edges, i, 0), b: end(mesh.edges, i, 1), drawn: owner.kind === "sketch", ...(flags ? { flags } : {}) };
}

function segDist(p: P2, a: P2, b: P2): number {
  const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
  const l2 = dx * dx + dy * dy;
  const t = l2 > 0 ? Math.max(0, Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2)) : 0;
  return Math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy);
}

/** What a pick becomes under the filter: null when nothing of that kind is near. */
export function filterPick(raw: Picked | null, filter: PickFilter, mesh: MeshData | null, toScreen: (p: V3) => P2, at: P2, px = FILTER_PX): Entity | null {
  if (!raw) return null;
  const e = raw.entity;
  if (filter === "all") return e;
  if (filter === "face") {
    if (e.kind === "face" || e.kind === "region") return e;
    const s = raw.surface?.at;
    return s ? { ...s } : null;
  }
  if (filter === "vertex" && e.kind === "vertex") return e;
  if (filter === "edge" && e.kind === "edge") return e;
  if (!mesh) return null;
  const owner = ownerOf(e, raw.surface);
  if (!owner) return null;
  let best: Entity | null = null;
  let bestD = px;
  for (const i of edgesOf(mesh, owner)) {
    if (filter === "vertex") {
      for (const k of [0, 1] as const) {
        const p = end(mesh.edges, i, k);
        const s = toScreen(p);
        const d = Math.hypot(s[0] - at[0], s[1] - at[1]);
        if (d < bestD) [best, bestD] = [{ kind: "vertex", owner, at: p }, d];
      }
    } else {
      const d = segDist(at, toScreen(end(mesh.edges, i, 0)), toScreen(end(mesh.edges, i, 1)));
      if (d < bestD) [best, bestD] = [edgeEntity(mesh, owner, i), d];
    }
  }
  return best;
}

/** The points of a mesh's solids and sketches, once each (for drawing them in Points). */
export function meshPoints(mesh: MeshData | null, max = 5000): V3[] {
  if (!mesh) return [];
  const seen = new Set<string>();
  const out: V3[] = [];
  for (let i = 0; i < mesh.edgeOwner.length && out.length < max; i++) {
    const o = mesh.edgeOwners[mesh.edgeOwner[i]];
    if (!o || o.kind === "mesh") continue;
    for (const k of [0, 1] as const) {
      const p = end(mesh.edges, i, k);
      const key = p.map((v) => v.toFixed(5)).join(",");
      if (!seen.has(key)) {
        seen.add(key);
        out.push(p);
      }
    }
  }
  return out;
}

/** A window (wholly inside) or crossing (touching) box under the filter. */
export function boxFilter(mesh: MeshData | null, toScreen: (p: V3) => P2, box: Box2, crossing: boolean, filter: Exclude<PickFilter, "all">): Entity[] {
  if (!mesh) return [];
  const b = norm(box);
  const out: Entity[] = [];
  if (filter === "face") {
    const pos = mesh.positions;
    const pt = (j: number): P2 => toScreen([pos[3 * j], pos[3 * j + 1], pos[3 * j + 2]]);
    const inside = new Map<number, boolean>();
    const touched = new Set<number>();
    for (let t = 0; t < mesh.faceIds.length; t++) {
      const part = mesh.faceIds[t];
      const [a, c, d] = [pt(mesh.indices[3 * t]), pt(mesh.indices[3 * t + 1]), pt(mesh.indices[3 * t + 2])];
      inside.set(part, (inside.get(part) ?? true) && inBox(b, a) && inBox(b, c) && inBox(b, d));
      if (crossing && triangleTouches(b, a, c, d)) touched.add(part);
    }
    mesh.parts.forEach((p, i) => {
      if (!(crossing ? touched.has(i) : inside.get(i))) return;
      if (p.kind === "face") out.push({ kind: "face", object: p.owner, face: p.index });
      else if (p.kind === "region") out.push({ kind: "region", sketch: p.owner, region: p.index });
    });
    return out;
  }
  const seen = new Set<string>();
  for (let i = 0; i < mesh.edgeOwner.length; i++) {
    const o = mesh.edgeOwners[mesh.edgeOwner[i]];
    if (!o || o.kind === "mesh") continue;
    const owner: Owner = { kind: o.kind, id: o.id };
    const [a, c] = [end(mesh.edges, i, 0), end(mesh.edges, i, 1)];
    if (filter === "vertex") {
      for (const p of [a, c]) {
        const key = `${o.kind}${o.id}:${p.map((v) => v.toFixed(5)).join(",")}`;
        if (!seen.has(key) && inBox(b, toScreen(p))) {
          seen.add(key);
          out.push({ kind: "vertex", owner, at: p });
        }
      }
    } else {
      const [sa, sc] = [toScreen(a), toScreen(c)];
      if (crossing ? segmentTouches(b, sa, sc) : inBox(b, sa) && inBox(b, sc)) out.push(edgeEntity(mesh, owner, i));
    }
  }
  return out;
}
