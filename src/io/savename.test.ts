// sketchcraft E-63: the name typed for Save As in a browser without a Save dialog.
import { describe, expect, it } from "vitest";
import { typedFileName } from "./files";

describe("typed file names (sketchcraft E-63)", () => {
  it("gets the extension once, keeps accents, makes unsafe characters safe", () => {
    expect(typedFileName("bâtiment 740C7886", ".shz")).toBe("bâtiment 740C7886.shz");
    expect(typedFileName("maison.SHZ", ".shz")).toBe("maison.SHZ");
    expect(typedFileName("  a/b:c*  ", ".shz")).toBe("a_b_c_.shz");
    expect(typedFileName("relevé.", ".shz")).toBe("relevé.shz");
  });

  it("refuses an empty name", () => {
    expect(typedFileName("   ", ".shz")).toBeNull();
    expect(typedFileName("...", ".shz")).toBeNull();
  });
});
