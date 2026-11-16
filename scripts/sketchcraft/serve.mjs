#!/usr/bin/env node
// sketchcraft E-24: serves the built app (dist/) on this computer only and opens it in the
// default browser. No dependency but Node: the launcher of an installed sketchcraft.
//
//   node scripts/sketchcraft/serve.mjs [dist folder] [--port 5180] [--no-open]
//
// It listens on 127.0.0.1 only (nothing is reachable from the network), on the first free
// port from 5180, with the MIME types the app needs (application/wasm for the kernel, ES
// modules for the workers). It stops when its window or terminal is closed, or on Ctrl+C.
import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const flag = (name) => args.includes(name);
const option = (name, fallback) => {
  const i = args.indexOf(name);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
};
const positional = args.filter((a, i) => !a.startsWith("--") && !(i > 0 && args[i - 1] === "--port"));
const root = path.resolve(positional[0] ?? path.join(here, "..", "..", "dist"));
const firstPort = Number(option("--port", "5180"));

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".webp": "image/webp",
  ".ico": "image/x-icon",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".ttf": "font/ttf",
  ".txt": "text/plain; charset=utf-8",
  ".map": "application/json; charset=utf-8",
};

/** The file under `root` for a URL path, or null when it is outside it or missing. */
export function fileFor(rootDir, urlPath) {
  let p;
  try {
    p = decodeURIComponent(urlPath.split("?")[0].split("#")[0]);
  } catch {
    return null;
  }
  const full = path.resolve(rootDir, "." + (p.endsWith("/") ? p + "index.html" : p));
  const rel = path.relative(rootDir, full);
  if (rel.startsWith("..") || path.isAbsolute(rel)) return null;
  try {
    return statSync(full).isFile() ? full : null;
  } catch {
    return null;
  }
}

function serve(rootDir) {
  return createServer((req, res) => {
    if (req.method !== "GET" && req.method !== "HEAD") {
      res.writeHead(405, { Allow: "GET, HEAD" }).end();
      return;
    }
    // A path the app does not have falls back to index.html (it is one page).
    const file = fileFor(rootDir, req.url ?? "/") ?? path.join(rootDir, "index.html");
    const type = TYPES[path.extname(file).toLowerCase()] ?? "application/octet-stream";
    res.writeHead(200, { "Content-Type": type, "Cache-Control": "no-cache", "X-Content-Type-Options": "nosniff" });
    if (req.method === "HEAD") return void res.end();
    createReadStream(file)
      .on("error", () => res.destroy())
      .pipe(res);
  });
}

function listen(server, port, tries = 20) {
  return new Promise((resolve, reject) => {
    server.once("error", (e) => {
      if (e.code === "EADDRINUSE" && tries > 1) resolve(listen(server, port + 1, tries - 1));
      else reject(e);
    });
    server.listen(port, "127.0.0.1", () => resolve(port));
  });
}

function openBrowser(url) {
  const cmd = process.platform === "win32" ? ["cmd", ["/c", "start", "", url]] : process.platform === "darwin" ? ["open", [url]] : ["xdg-open", [url]];
  try {
    spawn(cmd[0], cmd[1], { stdio: "ignore", detached: true }).unref();
  } catch {
    // The address is printed; it can be opened by hand.
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (!existsSync(path.join(root, "index.html"))) {
    console.error(`sketchcraft : pas d'application construite dans ${root} (index.html manque). Relancez l'installation.`);
    process.exit(1);
  }
  const server = serve(root);
  const port = await listen(server, firstPort).catch((e) => {
    console.error(`sketchcraft : impossible d'ouvrir le serveur local sur 127.0.0.1 : ${e.message}`);
    process.exit(1);
  });
  const url = `http://127.0.0.1:${port}/`;
  console.log(`sketchcraft fonctionne à l'adresse ${url}`);
  console.log("Laissez cette fenêtre ouverte pendant le travail ; fermez-la (ou Ctrl+C) pour arrêter sketchcraft.");
  if (!flag("--no-open")) openBrowser(url);
}
