// sketchcraft E-39 (H-20261117-16): COLLADA (.dae) import, for the models GeoHVAC exports
// (JC, 17 nov. 2026: « pour l'export actuel de geohvac c'est du dae mais volumecraft ne le
// supporte pas »). The file is read by three.js's COLLADA reader (its units and up axis
// applied), every mesh is baked into world coordinates with a plain colour, and the result is
// handed to the kernel's glTF reader as a .glb: the import then goes exactly as a glTF one
// (Y up in metres to the model's Z up, watertight meshes become solids). No image the file
// names is fetched.
import { Color, LoadingManager, Mesh, MeshStandardMaterial, Scene, type Material } from "three";
import { ColladaLoader } from "three/examples/jsm/loaders/ColladaLoader.js";
import { GLTFExporter } from "three/examples/jsm/exporters/GLTFExporter.js";

export interface DaeConverted {
  /** The .glb bytes for the kernel's glTF reader. */
  glb: Uint8Array;
  /** What the conversion did, for the import report. */
  notes: string[];
}

export async function daeToGlb(text: string): Promise<DaeConverted> {
  // Textures are not wanted: every URL the reader asks for becomes an empty data URL.
  const manager = new LoadingManager();
  manager.setURLModifier(() => "data:,");
  const loader = new ColladaLoader(manager);
  const warn = console.warn;
  let parsed: ReturnType<ColladaLoader["parse"]> | null;
  try {
    console.warn = () => {}; // its Z-up notice is expected here
    parsed = loader.parse(text, "");
  } finally {
    console.warn = warn;
  }
  if (!parsed?.scene) throw new Error("not a COLLADA file the reader understands");
  const src = parsed.scene;
  src.updateMatrixWorld(true);
  const out = new Scene();
  let meshes = 0;
  let triangles = 0;
  src.traverse((o) => {
    const m = o as Mesh;
    if (!m.isMesh || !m.geometry?.attributes?.position) return;
    const g = m.geometry.clone();
    g.applyMatrix4(m.matrixWorld);
    // Only positions (and the triangles): normals, UVs and colours are not needed.
    for (const name of Object.keys(g.attributes)) if (name !== "position") g.deleteAttribute(name);
    const mat = Array.isArray(m.material) ? m.material[0] : m.material;
    const colour = (mat as Material & { color?: Color })?.color ?? new Color(0.8, 0.8, 0.8);
    const baked = new Mesh(g, new MeshStandardMaterial({ color: colour.clone() }));
    baked.name = m.name || o.parent?.name || `mesh ${meshes + 1}`;
    out.add(baked);
    meshes++;
    triangles += (g.index ? g.index.count : g.attributes.position.count) / 3;
  });
  if (!meshes) throw new Error("the COLLADA file has no mesh to import");
  const data = await new GLTFExporter().parseAsync(out, { binary: true });
  const glb = new Uint8Array(data as ArrayBuffer);
  return { glb, notes: [`COLLADA read: ${meshes} mesh${meshes === 1 ? "" : "es"}, ${Math.round(triangles)} triangles (units and up axis of the file applied; textures left out)`] };
}
