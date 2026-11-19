// sketchcraft E-43 (H-20261118-1): Volume > Align to Axes. An imported building (GeoHVAC's
// .dae keep their real orientation) sits askew to red and green; this finds the turn about
// blue that lines its walls up with them. The walls' level edges vote, each by its length,
// on a direction modulo 90° (the mean of 4 × its angle), so walls at right angles agree and
// the longest walls weigh most.
import type { MeshData, V3 } from "../kernel/types";

/** Edge segments (xyz pairs) of the given solids in the mesh the viewport draws. */
export function solidEdges(mesh: MeshData, objects: ReadonlySet<number>): [V3, V3][] {
  const out: [V3, V3][] = [];
  for (let i = 0; i < mesh.edgeOwner.length; i++) {
    const o = mesh.edgeOwners[mesh.edgeOwner[i]];
    if (o?.kind !== "object" || !objects.has(o.id)) continue;
    const e = mesh.edges;
    out.push([
      [e[6 * i], e[6 * i + 1], e[6 * i + 2]],
      [e[6 * i + 3], e[6 * i + 4], e[6 * i + 5]],
    ]);
  }
  return out;
}

/** The walls' direction (radians, in (-45°, 45°]) from red: turning by minus this lines them
 * up with red and green. Null when there is no level edge to go by. */
export function wallAngle(edges: readonly [V3, V3][]): number | null {
  let c = 0;
  let s = 0;
  for (const [a, b] of edges) {
    const dx = b[0] - a[0];
    const dy = b[1] - a[1];
    const h = Math.hypot(dx, dy);
    if (h < 1e-6 || Math.abs(b[2] - a[2]) > 0.05 * h) continue; // level edges only
    const t = 4 * Math.atan2(dy, dx);
    c += h * Math.cos(t);
    s += h * Math.sin(t);
  }
  if (Math.hypot(c, s) < 1e-9) return null;
  return Math.atan2(s, c) / 4;
}

/** The centre of the edges' plan extent, on the ground (the turn's centre: the building
 * stays where it was). */
export function planCentre(edges: readonly [V3, V3][]): V3 {
  let [x0, y0, x1, y1, z0] = [Infinity, Infinity, -Infinity, -Infinity, Infinity];
  for (const e of edges)
    for (const p of e) {
      x0 = Math.min(x0, p[0]);
      y0 = Math.min(y0, p[1]);
      x1 = Math.max(x1, p[0]);
      y1 = Math.max(y1, p[1]);
      z0 = Math.min(z0, p[2]);
    }
  return [(x0 + x1) / 2, (y0 + y1) / 2, Number.isFinite(z0) ? z0 : 0];
}

/** sketchcraft E-48: the turn (radians, in (-45°, 45°]) that lines a façade up: its normal,
 * flattened onto the ground, measured from red modulo 90°, so turning by minus this makes the
 * façade face the nearest of red and green (either way). Null for a face that is nearly level
 * (a roof or floor has no façade direction). */
export function facadeAngle(normal: V3): number | null {
  const h = Math.hypot(normal[0], normal[1]);
  if (h < 0.2 * Math.hypot(normal[0], normal[1], normal[2])) return null;
  let a = Math.atan2(normal[1], normal[0]);
  const q = Math.PI / 2;
  a -= q * Math.round(a / q);
  if (a <= -Math.PI / 4 + 1e-12) a += q;
  return a;
}
