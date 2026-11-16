// How each inference looks: OUR colours (ADR-0003), never SketchUp's. The four point/face
// colours are the approved mock's (endpoint lime, midpoint teal, on-edge amber, on-face
// violet); the rest extend the same palette. Values live in CSS tokens (styles/tokens.css)
// so light and dark themes can differ; the Rust inference crate names the same tokens.

export type InferenceKind =
  | "endpoint"
  | "midpoint"
  | "on-edge"
  | "on-face"
  | "intersection"
  | "centre"
  | "axis-x"
  | "axis-y"
  | "axis-z"
  | "parallel"
  | "perpendicular"
  | "from-point"
  | "on-guide"
  | "guide-point"
  | "grid"
  | "free";

export interface InferenceStyle {
  label: string;
  /** CSS custom property holding the colour. */
  token: string;
}

export const INFERENCE_STYLES: Record<InferenceKind, InferenceStyle> = {
  endpoint: { label: "Endpoint", token: "--inf-end" },
  midpoint: { label: "Midpoint", token: "--inf-mid" },
  "on-edge": { label: "On Edge", token: "--inf-edge" },
  "on-face": { label: "On Face", token: "--inf-face" },
  intersection: { label: "Intersection", token: "--inf-int" },
  centre: { label: "Centre", token: "--inf-ctr" },
  "axis-x": { label: "On Red Axis", token: "--axis-x" },
  "axis-y": { label: "On Green Axis", token: "--axis-y" },
  "axis-z": { label: "On Blue Axis", token: "--axis-z" },
  parallel: { label: "Parallel to Edge", token: "--inf-par" },
  perpendicular: { label: "Perpendicular to Edge", token: "--inf-par" },
  "from-point": { label: "From Point", token: "--inf-from" },
  "on-guide": { label: "On Guide", token: "--inf-guide" },
  "guide-point": { label: "Guide Point", token: "--inf-guide" },
  grid: { label: "Grid", token: "--inf-grid" },
  free: { label: "", token: "--muted" },
};

/** The legend under the viewport shows the mock's four. */
export const LEGEND_KINDS: InferenceKind[] = ["endpoint", "midpoint", "on-edge", "on-face"];

/**
 * sketchcraft E-16: the legend's switches. Each turns its kinds off for snapping (the next
 * kind down takes over); "grid" is View > Snap to Grid. On Face is not one: drawing needs
 * the surface under the cursor.
 */
export interface SnapSwitch {
  id: string;
  label: string;
  token: string;
  kinds: InferenceKind[];
}
export const SNAP_SWITCHES: SnapSwitch[] = [
  { id: "endpoint", label: "Endpoint", token: "--inf-end", kinds: ["endpoint"] },
  { id: "midpoint", label: "Midpoint", token: "--inf-mid", kinds: ["midpoint"] },
  { id: "on-edge", label: "On Edge", token: "--inf-edge", kinds: ["on-edge"] },
  { id: "intersection", label: "Intersection", token: "--inf-int", kinds: ["intersection"] },
  { id: "centre", label: "Centre", token: "--inf-ctr", kinds: ["centre"] },
  { id: "axes", label: "Axes", token: "--axis-z", kinds: ["axis-x", "axis-y", "axis-z"] },
  { id: "parallel", label: "Parallel / Perp.", token: "--inf-par", kinds: ["parallel", "perpendicular"] },
  { id: "from-point", label: "From Point", token: "--inf-from", kinds: ["from-point"] },
  { id: "guides", label: "Guides", token: "--inf-guide", kinds: ["on-guide", "guide-point"] },
  { id: "grid", label: "Grid", token: "--inf-grid", kinds: ["grid"] },
];

/** The kinds the switched-off ids turn off ("grid" is not among them: Snap to Grid). */
export function disabledKinds(off: readonly string[]): InferenceKind[] {
  return SNAP_SWITCHES.filter((s) => s.id !== "grid" && off.includes(s.id)).flatMap((s) => s.kinds);
}

/** Inferences that constrain direction (drawn as a line from the gesture start). */
export function isLineInference(k: InferenceKind): boolean {
  return k === "axis-x" || k === "axis-y" || k === "axis-z" || k === "parallel" || k === "perpendicular" || k === "from-point";
}
