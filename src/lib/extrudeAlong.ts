// sketchcraft E-30 (H-20261117-7): what Volume > Extrude Faces Along Axis reads: a length,
// along blue unless a red/green/blue (or x/y/z) axis comes first; negative goes the other way.
import { parseLength, type UnitSystem } from "./units";

const AXES: Record<string, [number, number, number]> = {
  x: [1, 0, 0],
  red: [1, 0, 0],
  y: [0, 1, 0],
  green: [0, 1, 0],
  z: [0, 0, 1],
  blue: [0, 0, 1],
};

export type AlongResult = { ok: true; vector: [number, number, number] } | { ok: false; error: string };

export function parseAlong(text: string, units: UnitSystem): AlongResult {
  const m = /^\s*(x|y|z|red|green|blue)\b\s*[:=]?\s*(.*)$/i.exec(text);
  const axis = AXES[(m?.[1] ?? "z").toLowerCase()];
  const len = parseLength(m ? m[2] : text, units);
  if (!len.ok) return { ok: false, error: len.error };
  if (Math.abs(len.metres) < 1e-9) return { ok: false, error: "Type a distance other than zero." };
  const v = axis.map((a) => (a === 0 ? 0 : a * len.metres)) as [number, number, number];
  return { ok: true, vector: v };
}
