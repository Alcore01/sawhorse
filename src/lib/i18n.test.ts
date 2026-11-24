// sketchcraft E-59: the interface in French.
import { afterEach, describe, expect, it } from "vitest";
import { getLang, setLang, t, tChip } from "./i18n";

afterEach(() => setLang("en"));

describe("interface language (sketchcraft E-59)", () => {
  it("English by default here, and the internal tag never shows", () => {
    expect(getLang()).toBe("en");
    expect(t("Copy")).toBe("Copy");
    expect(t("sketchcraft: the selected faces, if they close a volume, become one solid")).toBe("the selected faces, if they close a volume, become one solid");
  });

  it("menus, tools and panes in French", () => {
    setLang("fr");
    expect(t("Edit")).toBe("Édition");
    expect(t("Push")).toBe("Pousser");
    expect(t("Tape")).toBe("Mètre");
    expect(t("Align Façade to Axes")).toBe("Aligner la façade sur les axes");
    expect(t("sketchcraft: the selected faces, if they close a volume, become one solid")).toBe("Les faces sélectionnées, si elles ferment un volume, deviennent un volume");
  });

  it("sentences with numbers and names", () => {
    setLang("fr");
    expect(t("Select.")).toBe("Sélection.");
    expect(t("Selected Solid 3.")).toBe("Sélectionné : volume 3.");
    expect(t("Copied the outline of a face. Edit > Paste (Ctrl+V), then click where it goes.")).toBe("Copié : le contour d'une face. Édition > Coller (Ctrl+V), puis cliquez où le poser.");
    expect(t("that point stays: it ends an edge (erase the edge instead)")).toBe("ce point reste : il termine une arête (effacez l'arête)");
    expect(t("SW · 225.0° from north")).toBe("SO · 225,0° depuis le nord");
  });

  it("the HUD, part by part", () => {
    setLang("fr");
    expect(tChip("1 solid · 6 faces · genus 0 · 158.4 in³ · watertight")).toBe("1 volume · 6 faces · genre 0 · 158.4 in³ · étanche");
    expect(tChip("Edit: Model · undo 3 · redo 0")).toBe("Édition : Modèle · annuler 3 · rétablir 0");
  });

  it("what is not translated yet stays in English", () => {
    setLang("fr");
    expect(t("Some message nobody translated yet")).toBe("Some message nobody translated yet");
  });
});

describe("north in French (sketchcraft E-62)", () => {
  it("the compass letters and the turn message", () => {
    setLang("fr");
    expect(t("W")).toBe("O");
    expect(t("Turned 20.00° clockwise about blue: the walls run along red and green. North turned with it (20.00° from green; the compass rose shows it); Undo turns both back.")).toContain("le nord a tourné avec (20.00° depuis le vert");
  });
});
