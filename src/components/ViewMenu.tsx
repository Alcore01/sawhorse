// sketchcraft E-57: the view's right-click menu (its items: src/lib/viewMenu.ts).
import { t } from "../lib/i18n";
import { useEffect, useRef } from "react";
import type { MenuItem } from "./TopBar";

export function ViewMenu({ x, y, items, onClose }: { x: number; y: number; items: MenuItem[]; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const close = (e: Event) => {
      if (ref.current && e.target instanceof Node && ref.current.contains(e.target)) return;
      onClose();
    };
    const esc = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("pointerdown", close);
    window.addEventListener("keydown", esc);
    return () => {
      window.removeEventListener("pointerdown", close);
      window.removeEventListener("keydown", esc);
    };
  }, [onClose]);
  // Kept on screen: opened near the right or bottom edge, it opens to the left or upwards.
  const left = Math.min(x, (typeof window === "undefined" ? 1e4 : window.innerWidth) - 230);
  const top = Math.min(y, (typeof window === "undefined" ? 1e4 : window.innerHeight) - (items.length * 30 + 16));
  return (
    <div className="ctxmenu view-menu" ref={ref} role="menu" aria-label="Actions" data-testid="view-menu" style={{ left, top }} onContextMenu={(e) => e.preventDefault()}>
      {items.map((it, i) =>
        it === "-" ? (
          <hr key={i} />
        ) : "items" in it ? null : (
          <button
            key={i}
            type="button"
            role="menuitem"
            title={it.title ? t(it.title) : undefined}
            onClick={() => {
              onClose();
              it.onSelect();
            }}
          >
            {t(it.label)}
            {it.keys ? <kbd>{it.keys}</kbd> : null}
          </button>
        ),
      )}
    </div>
  );
}
