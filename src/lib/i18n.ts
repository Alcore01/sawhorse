// sketchcraft E-59: the interface language (JC, 24 nov. 2026 : « il faudrait pouvoir changer
// la langue et mettre en français »). The English text is the key: `t()` gives the chosen
// language's text where the dictionary has it (exact phrases, then patterns for sentences that
// carry numbers or names), and the English otherwise, so nothing breaks while translation
// grows. Applied where text is shown (menus, the tool rail and hints, the status line, the HUD,
// the panes), not at each of the hundreds of places that write it.
import { FR, FR_PATTERNS, FR_WORDS } from "./i18n-fr";

export type Lang = "en" | "fr";

export const LANGS: { id: Lang; name: string }[] = [
  { id: "en", name: "English" },
  { id: "fr", name: "Français" },
];

const KEY = "volumecraft.lang";

function initial(): Lang {
  try {
    const v = localStorage.getItem(KEY);
    if (v === "en" || v === "fr") return v;
  } catch {
    /* storage blocked */
  }
  const nav = typeof navigator === "undefined" ? "" : (navigator.language ?? "");
  return nav.toLowerCase().startsWith("fr") ? "fr" : "en";
}

let lang: Lang = initial();

export function getLang(): Lang {
  return lang;
}

/** Choose the language (kept for the next sessions). The app reloads its text by rendering
 * again; callers that cache text should re-read it. */
export function setLang(l: Lang): void {
  lang = l;
  try {
    localStorage.setItem(KEY, l);
  } catch {
    /* storage blocked: for this session */
  }
}

/** The internal tag some tooltips start with (not for the reader). */
const strip = (s: string) => s.replace(/^sketchcraft:\s*/, "");

/** Words and names inside a sentence ("Solid 3", "face 2", "red"), after the sentence. */
function words(s: string): string {
  let out = s;
  for (const [re, to] of FR_WORDS) out = out.replace(re, to);
  return out;
}

/** `s` in the chosen language (English when there is no translation yet). */
export function t(s: string): string {
  const k = strip(s);
  if (lang === "en" || !k) return k;
  const hit = FR[k];
  if (hit !== undefined) return hit;
  for (const [re, to] of FR_PATTERNS) {
    const m = k.match(re);
    if (m) return words(typeof to === "string" ? k.replace(re, to) : to(m, t));
  }
  return k;
}

/** A HUD chip ("1 solid · 6 faces · watertight"): each part on its own. */
export function tChip(s: string): string {
  if (lang === "en") return s;
  return s
    .split(" · ")
    .map((p) => t(p))
    .join(" · ");
}
