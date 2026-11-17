// sketchcraft E-39: a GeoHVAC-like COLLADA file (Z up, centimetres) becomes a .glb in metres,
// Y up, ready for the kernel's glTF reader.
import { describe, expect, it } from "vitest";
import { daeToGlb } from "./dae";

/** A 2 x 1 x 1 m box, written in centimetres, Z up: 8 corners, 12 triangles. */
const BOX = `<?xml version="1.0" encoding="utf-8"?>
<COLLADA xmlns="http://www.collada.org/2005/11/COLLADASchema" version="1.4.1">
  <asset><unit name="centimeter" meter="0.01"/><up_axis>Z_UP</up_axis></asset>
  <library_geometries>
    <geometry id="box" name="building">
      <mesh>
        <source id="box-pos">
          <float_array id="box-pos-array" count="24">0 0 0 200 0 0 200 100 0 0 100 0 0 0 100 200 0 100 200 100 100 0 100 100</float_array>
          <technique_common><accessor source="#box-pos-array" count="8" stride="3">
            <param name="X" type="float"/><param name="Y" type="float"/><param name="Z" type="float"/>
          </accessor></technique_common>
        </source>
        <vertices id="box-vtx"><input semantic="POSITION" source="#box-pos"/></vertices>
        <triangles count="12">
          <input semantic="VERTEX" source="#box-vtx" offset="0"/>
          <p>0 2 1 0 3 2 4 5 6 4 6 7 0 1 5 0 5 4 1 2 6 1 6 5 2 3 7 2 7 6 3 0 4 3 4 7</p>
        </triangles>
      </mesh>
    </geometry>
  </library_geometries>
  <library_visual_scenes>
    <visual_scene id="scene"><node id="n"><instance_geometry url="#box"/></node></visual_scene>
  </library_visual_scenes>
  <scene><instance_visual_scene url="#scene"/></scene>
</COLLADA>`;

/** The positions of a .glb's first mesh (float32 xyz), read straight from its chunks. */
function positions(glb: Uint8Array): number[][] {
  const dv = new DataView(glb.buffer, glb.byteOffset, glb.byteLength);
  const jsonLen = dv.getUint32(12, true);
  const json = JSON.parse(new TextDecoder().decode(glb.subarray(20, 20 + jsonLen)));
  const bin = 20 + jsonLen + 8;
  const acc = json.accessors[json.meshes[0].primitives[0].attributes.POSITION];
  const view = json.bufferViews[acc.bufferView];
  const start = bin + (view.byteOffset ?? 0) + (acc.byteOffset ?? 0);
  const out: number[][] = [];
  for (let i = 0; i < acc.count; i++) out.push([0, 1, 2].map((k) => dv.getFloat32(start + (i * 3 + k) * 4, true)));
  return out;
}

describe("COLLADA import (E-39)", () => {
  it("becomes a .glb in metres, Y up, with a note", async () => {
    const r = await daeToGlb(BOX);
    expect(new TextDecoder().decode(r.glb.subarray(0, 4))).toBe("glTF");
    const p = positions(r.glb);
    const span = (k: number) => Math.max(...p.map((v) => v[k])) - Math.min(...p.map((v) => v[k]));
    // 2 m along x; the 1 m height (Z in the file) is Y in glTF; 1 m deep along -Z.
    expect(span(0)).toBeCloseTo(2, 5);
    expect(span(1)).toBeCloseTo(1, 5);
    expect(span(2)).toBeCloseTo(1, 5);
    expect(Math.min(...p.map((v) => v[1]))).toBeCloseTo(0, 5);
    expect(r.notes[0]).toMatch(/1 mesh, 12 triangles/);
  });

  it("refuses what is not COLLADA", async () => {
    await expect(daeToGlb("<html></html>")).rejects.toThrow();
  });
});
