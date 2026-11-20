import { describe, expect, it } from "vitest";
import type { MenuItem } from "../components/TopBar";
import { viewMenuItems, viewMenuLabels } from "./viewMenu";

const go = () => {};
const menus: Record<string, MenuItem[]> = {
  Edit: [{ label: "Delete", onSelect: go }, "-", { label: "Make Group", onSelect: go }, { label: "Hide", onSelect: go }, { label: "Lock", disabled: true, onSelect: go }],
  Volume: [{ label: "Merge Faces", disabled: true, onSelect: go }, { label: "Align Façade to Axes", onSelect: go }, { label: "Auto Merge Faces", onSelect: go }, { label: "Clean Up Points", onSelect: go }],
};

describe("view right-click menu (sketchcraft E-57)", () => {
  it("offers a face's commands, only those that can run now, from the menu bar", () => {
    const items = viewMenuItems(viewMenuLabels(new Set(["face"])), menus, { Properties: { label: "Properties", onSelect: go } });
    expect(items.map((i) => (i === "-" ? "-" : i.label))).toEqual(["Delete", "-", "Align Façade to Axes", "Auto Merge Faces", "Clean Up Points", "-", "Make Group", "Hide", "-", "Properties"]);
  });

  it("with nothing selected: the whole-model commands", () => {
    expect(viewMenuLabels(new Set())).toContain("Align to Axes");
    expect(viewMenuLabels(new Set())).not.toContain("Delete");
  });

  it("never starts, ends or doubles a separator", () => {
    const items = viewMenuItems(["-", "Lock", "-", "-", "Hide", "-"], menus);
    expect(items.map((i) => (i === "-" ? "-" : i.label))).toEqual(["Hide"]);
  });
});
