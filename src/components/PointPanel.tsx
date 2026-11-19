// sketchcraft E-49: the selection filter switch (All, Points, Edges, Faces) and the Point
// panel: the X, Y, Z of the one selected point, each editable (Enter moves the point there;
// only the field typed in changes, so a rounded display never nudges the others).
import { useEffect, useState } from "react";
import type { V3 } from "../kernel/types";
import { formatLength, parseLength, type UnitSystem } from "../lib/units";
import type { PickFilter } from "../lib/pickFilter";

const FILTERS: { id: PickFilter; label: string; title: string }[] = [
  { id: "all", label: "All", title: "Select anything: what is under the cursor" },
  { id: "vertex", label: "Points", title: "Select points only (the corners of the solids, shown as dots); one point selected shows its X, Y, Z to edit" },
  { id: "edge", label: "Edges", title: "Select edges only: the nearest edge of the solid under the cursor" },
  { id: "face", label: "Faces", title: "Select faces only: the face under the cursor" },
];

export function PickFilterSwitch({ filter, onFilter }: { filter: PickFilter; onFilter: (f: PickFilter) => void }) {
  return (
    <div className="mode-switch pick-filter" role="radiogroup" aria-label="Selection filter">
      {FILTERS.map((f) => (
        <button key={f.id} type="button" role="radio" aria-checked={filter === f.id} data-filter={f.id} onClick={() => onFilter(f.id)} title={f.title}>
          {f.label}
        </button>
      ))}
    </div>
  );
}

const shown = (m: number, units: UnitSystem) => formatLength(m, units).replace(/^~\s*/, "");

export interface PointPanelProps {
  at: V3;
  units: UnitSystem;
  /** Move the point to `to` (world metres). */
  onMove: (to: V3) => void;
  onError: (text: string) => void;
}

export function PointPanel({ at, units, onMove, onError }: PointPanelProps) {
  const [text, setText] = useState<string[]>(() => at.map((v) => shown(v, units)));
  const key = `${at.join(",")}|${units}`;
  useEffect(() => setText(at.map((v) => shown(v, units))), [key]); // eslint-disable-line react-hooks/exhaustive-deps
  const commit = (i: number) => {
    if (text[i] === shown(at[i], units)) return; // untouched: no move, no undo step
    const r = parseLength(text[i], units);
    if (!r.ok) return onError(r.error);
    if (Math.abs(r.metres - at[i]) <= 1e-9) return;
    const to: V3 = [...at];
    to[i] = r.metres;
    onMove(to);
  };
  return (
    <div className="point-panel" data-testid="point-panel" role="group" aria-label="Selected point">
      <strong>Point</strong>
      {(["X", "Y", "Z"] as const).map((n, i) => (
        <label key={n} className={`axis-${n.toLowerCase()}`}>
          {n}
          <input
            value={text[i]}
            aria-label={`Point ${n}`}
            onChange={(e) => setText((t) => t.map((v, j) => (j === i ? e.target.value : v)))}
            onKeyDown={(e) => {
              if (e.key === "Enter") commit(i);
              if (e.key === "Escape") setText(at.map((v) => shown(v, units)));
              e.stopPropagation();
            }}
            onBlur={() => commit(i)}
          />
        </label>
      ))}
    </div>
  );
}
