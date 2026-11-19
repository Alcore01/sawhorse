// sketchcraft E-49: the Point panel moves the selected point to typed coordinates.
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { PickFilterSwitch, PointPanel } from "./PointPanel";

afterEach(cleanup);

describe("Point panel", () => {
  it("shows X, Y, Z and moves only along the field typed in", () => {
    const onMove = vi.fn();
    // y is shown rounded (~ 1.3333): untouched, it must not nudge the point.
    render(<PointPanel at={[2, 4 / 3, 2.5]} units="m" onMove={onMove} onError={() => {}} />);
    const z = screen.getByLabelText("Point Z") as HTMLInputElement;
    expect(z.value).toBe("2.5");
    fireEvent.change(z, { target: { value: "3.1" } });
    fireEvent.keyDown(z, { key: "Enter" });
    expect(onMove).toHaveBeenCalledWith([2, 4 / 3, 3.1]);
  });

  it("an untouched field moves nothing; a bad entry is reported", () => {
    const onMove = vi.fn();
    const onError = vi.fn();
    render(<PointPanel at={[0, 0, 0]} units="cm" onMove={onMove} onError={onError} />);
    const x = screen.getByLabelText("Point X");
    fireEvent.blur(x);
    expect(onMove).not.toHaveBeenCalled();
    fireEvent.change(x, { target: { value: "abc" } });
    fireEvent.keyDown(x, { key: "Enter" });
    expect(onMove).not.toHaveBeenCalled();
    expect(onError).toHaveBeenCalled();
  });

  it("the filter switch says which kind is on", () => {
    const onFilter = vi.fn();
    render(<PickFilterSwitch filter="vertex" onFilter={onFilter} />);
    expect(screen.getByRole("radio", { name: "Points" }).getAttribute("aria-checked")).toBe("true");
    fireEvent.click(screen.getByRole("radio", { name: "Faces" }));
    expect(onFilter).toHaveBeenCalledWith("face");
  });
});
