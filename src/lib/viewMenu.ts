// sketchcraft E-57: the view's right-click menu (JC, 20 nov. 2026, « Menu clic droit » ; the
// idea seen in SketchCraft, not its code): the commands that apply to what is under the
// cursor or selected, taken from the menu bar's own entries (same labels, same actions, same
// rules for when they can run), so the two never disagree.
import type { MenuItem } from "../components/TopBar";
import type { Entity } from "../kernel/types";

type Kind = Entity["kind"];

/** The labels offered for a selection of these kinds ("-" between groups), best first. */
export function viewMenuLabels(kinds: ReadonlySet<Kind>): string[] {
  if (!kinds.size) return ["Select All", "-", "Align to Axes", "Clean Up Points", "Unify Imported Meshes", "-", "Properties"];
  const out: string[] = [];
  const add = (...l: string[]) => {
    for (const x of l) if (x === "-" || !out.includes(x)) out.push(x);
  };
  add("Delete", "-");
  if (kinds.has("face")) add("Align Façade to Axes", "Merge Faces", "Auto Merge Faces", "Clean Up Points");
  if (kinds.has("object")) add("Join Solids", "Explode Solid into Faces", "Align to Axes", "Auto Merge Faces", "Clean Up Points");
  if (kinds.has("region")) add("Make Solid from Faces", "Extrude Faces Along Axis…", "Clean Up Points");
  if (kinds.has("vertex") || kinds.has("edge")) add("Clean Up Points");
  if (kinds.has("node")) add("Explode", "Make Unique");
  add("-", "Make Group", "Hide", "Lock", "-", "Properties");
  return out;
}

type Action = Extract<MenuItem, { onSelect: () => void }>;

/** Every runnable entry of the menu bar, by label (sub-menus too). */
function byLabel(menus: Record<string, MenuItem[]>): Map<string, Action> {
  const m = new Map<string, Action>();
  const walk = (items: MenuItem[]) => {
    for (const it of items) {
      if (it === "-") continue;
      if ("items" in it) walk(it.items);
      else if (!m.has(it.label)) m.set(it.label, it);
    }
  };
  for (const items of Object.values(menus)) walk(items);
  return m;
}

/** The menu: the labels' menu-bar entries that can run now, `extra` for labels the menu bar
 * has not (Properties), no separator first, last or twice. */
export function viewMenuItems(labels: string[], menus: Record<string, MenuItem[]>, extra: Record<string, Action> = {}): MenuItem[] {
  const all = byLabel(menus);
  const out: MenuItem[] = [];
  for (const l of labels) {
    if (l === "-") {
      if (out.length && out[out.length - 1] !== "-") out.push("-");
      continue;
    }
    const it = extra[l] ?? all.get(l);
    if (it && !it.disabled) out.push(it);
  }
  while (out[out.length - 1] === "-") out.pop();
  return out;
}
