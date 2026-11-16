// sketchcraft E-15: View > Snap to Grid reaches the inference query.
import { describe, expect, it } from "vitest";
import type { KernelClient } from "../kernel/client";
import type { InferQuery } from "../kernel/types";
import type { ViewportEngine } from "../viewport/engine";
import { kernelResolver } from "./driver";

const M = { shift: false, ctrl: false, alt: false };

function harness(snap: boolean) {
  const seen: InferQuery[] = [];
  const kernel = { infer: async (q: InferQuery) => (seen.push(q), null) } as unknown as KernelClient;
  const engine = {
    ray: () => ({ origin: [0, 0, 10], dir: [0, 0, -1] }),
    pixelScale: () => ({ ortho: true, scale: 0.001, eye: [0, 0, 10], forward: [0, 0, -1] }),
  } as unknown as ViewportEngine;
  const resolve = kernelResolver(() => kernel, () => engine, () => [0.01, 0.1], undefined, undefined, () => snap);
  return { seen, resolve };
}

describe("Snap to Grid", () => {
  it("off: no grid points and no grid-step lengths, even for a tool that asks for them", async () => {
    const { seen, resolve } = harness(false);
    await resolve(0, 0, {}, M);
    await resolve(0, 0, { grid: true }, M);
    expect(seen.map((q) => q.grid)).toEqual([false, false]);
  });

  it("on: as each tool asks (Offset and Scale keep it off for themselves)", async () => {
    const { seen, resolve } = harness(true);
    await resolve(0, 0, {}, M);
    await resolve(0, 0, { grid: false }, M);
    expect(seen.map((q) => q.grid)).toEqual([true, false]);
  });
});
