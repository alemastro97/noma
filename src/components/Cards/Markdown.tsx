import { useMemo, type ReactNode } from "react";
import { openLink } from "../../api/cards";
import { parseMarkdown, type Inline, type ListEntry } from "../../lib/markdown";
import { toastError } from "../../stores/toasts";
import styles from "./Cards.module.css";

function inline(nodes: Inline[]): ReactNode[] {
  return nodes.map((n, i) => {
    switch (n.t) {
      case "text":
        return n.v;
      case "br":
        return <br key={i} />;
      case "code":
        return (
          <code key={i} className={styles.mdCode}>
            {n.v}
          </code>
        );
      case "strong":
        return <strong key={i}>{inline(n.c)}</strong>;
      case "em":
        return <em key={i}>{inline(n.c)}</em>;
      case "del":
        return <del key={i}>{inline(n.c)}</del>;
      case "link":
        return (
          <a
            key={i}
            href={n.href}
            className={styles.mdLink}
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              openLink(n.href).catch((err: unknown) => toastError(String(err)));
            }}
          >
            {inline(n.c)}
          </a>
        );
    }
  });
}

function ListItems({ items }: { items: ListEntry[] }) {
  return items.map((it, i) => (
    <li key={i} data-task={it.checked === null ? undefined : it.checked}>
      {it.checked !== null && (
        <span className={styles.mdTask} data-checked={it.checked || undefined} aria-hidden="true" />
      )}
      {inline(it.c)}
    </li>
  ));
}

/** Renders a card's notes as a safe Markdown subset (React elements only, no HTML). */
export function Markdown({ text }: { text: string }) {
  const blocks = useMemo(() => parseMarkdown(text), [text]);
  return (
    <div className={styles.markdown}>
      {blocks.map((b, i) => {
        switch (b.t) {
          case "p":
            return <p key={i}>{inline(b.c)}</p>;
          case "h": {
            const Tag = (["h3", "h4", "h5"] as const)[b.level - 1]!;
            return <Tag key={i}>{inline(b.c)}</Tag>;
          }
          case "ul":
            return (
              <ul key={i}>
                <ListItems items={b.items} />
              </ul>
            );
          case "ol":
            return (
              <ol key={i} start={b.start}>
                <ListItems items={b.items} />
              </ol>
            );
          case "quote":
            return <blockquote key={i}>{inline(b.c)}</blockquote>;
          case "code":
            return (
              <pre key={i} className={styles.mdPre}>
                <code>{b.v}</code>
              </pre>
            );
          case "hr":
            return <hr key={i} />;
        }
      })}
    </div>
  );
}
