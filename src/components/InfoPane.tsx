// sketchcraft E-56: the Info pane: what the selection measures (src/lib/properties.ts).
import type { InfoSheet } from "../lib/properties";

export interface InfoPaneProps {
  sheet: InfoSheet;
}

export function InfoPane({ sheet }: InfoPaneProps) {
  return (
    <div className="info-pane" data-testid="info-pane">
      <h4>{sheet.title}</h4>
      {sheet.rows.length ? (
        <dl>
          {sheet.rows.map((r, i) => (
            <div key={i} className="info-row">
              <dt>{r.label}</dt>
              <dd>{r.value}</dd>
            </div>
          ))}
        </dl>
      ) : null}
      {sheet.note ? <p className="note">{sheet.note}</p> : null}
    </div>
  );
}
