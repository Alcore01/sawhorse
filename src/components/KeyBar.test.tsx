// sketchcraft E-34: the on-screen keys send the app what a keyboard would.
import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { KeyBar } from "./KeyBar";

describe("KeyBar (E-34)", () => {
  const seen: string[] = [];
  const on = (e: KeyboardEvent) => seen.push(`${e.type}:${e.key}${e.ctrlKey ? "+ctrl" : ""}`);
  afterEach(() => {
    window.removeEventListener("keydown", on);
    window.removeEventListener("keyup", on);
    seen.length = 0;
  });

  it("sends Esc, Enter, the arrows, a held Shift and Ctrl+Z", () => {
    window.addEventListener("keydown", on);
    window.addEventListener("keyup", on);
    const close = vi.fn();
    render(<KeyBar onClose={close} />);
    for (const name of [/Escape/, /Enter/, /red axis/, /green axis/, /blue axis/, /parallel/, /^Shift$/, /^Shift$/, /^Undo$/]) {
      fireEvent.pointerDown(screen.getAllByRole("button", { name })[0]);
    }
    expect(seen).toEqual(["keydown:Escape", "keydown:Enter", "keydown:ArrowRight", "keydown:ArrowLeft", "keydown:ArrowUp", "keydown:ArrowDown", "keydown:Shift", "keyup:Shift", "keydown:z+ctrl"]);
    fireEvent.pointerDown(screen.getByRole("button", { name: /Hide/ }));
    expect(close).toHaveBeenCalled();
  });
});
