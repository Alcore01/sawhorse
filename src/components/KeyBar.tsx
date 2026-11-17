// sketchcraft E-34 (H-20261117-11): the keys a tablet with a pen has not got, as buttons over
// the view: Esc, Enter, the axis locks (the arrows: red, green, blue, parallel), Shift (held
// down until tapped again) and Undo. Each button sends the app the key a keyboard would, so
// the tools behave exactly as with a keyboard. Shown by View > On-screen Keys, and turned on
// by itself the first time a pen or a finger touches the app.
import { useState } from "react";

export function sendKey(key: string, type: "keydown" | "keyup" = "keydown", extra: KeyboardEventInit = {}): void {
  window.dispatchEvent(new KeyboardEvent(type, { key, bubbles: true, cancelable: true, ...extra }));
}

const KEYS: { label: string; key: string; title: string; className?: string }[] = [
  { label: "Esc", key: "Escape", title: "Escape: cancel what is under way" },
  { label: "↵", key: "Enter", title: "Enter: finish, or apply the typed value" },
  { label: "X", key: "ArrowRight", title: "Lock to the red axis (Right arrow)", className: "red" },
  { label: "Y", key: "ArrowLeft", title: "Lock to the green axis (Left arrow)", className: "green" },
  { label: "Z", key: "ArrowUp", title: "Lock to the blue axis (Up arrow)", className: "blue" },
  { label: "∥", key: "ArrowDown", title: "Lock to the parallel or perpendicular shown (Down arrow)" },
];

export function KeyBar({ onClose }: { onClose: () => void }) {
  const [shift, setShift] = useState(false);
  // Buttons never take the focus (the pen stays on the model) and act on the press.
  const press = (fn: () => void) => (e: React.PointerEvent) => {
    e.preventDefault();
    e.stopPropagation();
    fn();
  };
  return (
    <div className="key-bar" role="toolbar" aria-label="On-screen keys">
      {KEYS.map((k) => (
        <button key={k.key} type="button" className={k.className} title={k.title} aria-label={k.title} onPointerDown={press(() => sendKey(k.key))}>
          {k.label}
        </button>
      ))}
      <button
        type="button"
        aria-pressed={shift}
        title="Shift: held down until tapped again (locks the line shown, adds to the selection)"
        aria-label="Shift"
        onPointerDown={press(() => {
          sendKey("Shift", shift ? "keyup" : "keydown", { shiftKey: !shift });
          setShift(!shift);
        })}
      >
        ⇧
      </button>
      <button type="button" title="Undo (Ctrl+Z)" aria-label="Undo" onPointerDown={press(() => sendKey("z", "keydown", { ctrlKey: true }))}>
        ↶
      </button>
      <button type="button" className="close" title="Hide the on-screen keys (View > On-screen Keys)" aria-label="Hide the on-screen keys" onPointerDown={press(onClose)}>
        ×
      </button>
    </div>
  );
}
