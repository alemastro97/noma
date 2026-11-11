/**
 * A small, safe Markdown subset for card notes. It parses into a plain tree
 * that is rendered as React elements (never as HTML), so nothing in a note can
 * inject markup or script.
 *
 * Blocks: paragraphs, # headings (1-3), - / * / + lists, 1. lists, - [ ] task
 * items, > quotes, ``` code fences, --- rules.
 * Inline: **bold**, *italic* / _italic_, ~~strike~~, `code`, [text](url), bare URLs.
 */

export type Inline =
  | { t: "text"; v: string }
  | { t: "code"; v: string }
  | { t: "strong"; c: Inline[] }
  | { t: "em"; c: Inline[] }
  | { t: "del"; c: Inline[] }
  | { t: "link"; href: string; c: Inline[] }
  | { t: "br" };

export interface ListEntry {
  /** `null` for a plain item, `true`/`false` for a task item. */
  checked: boolean | null;
  c: Inline[];
}

export type Block =
  | { t: "p"; c: Inline[] }
  | { t: "h"; level: 1 | 2 | 3; c: Inline[] }
  | { t: "ul"; items: ListEntry[] }
  | { t: "ol"; start: number; items: ListEntry[] }
  | { t: "quote"; c: Inline[] }
  | { t: "code"; v: string }
  | { t: "hr" };

/** Only these schemes become clickable links. */
export function isSafeHref(href: string): boolean {
  return /^(https?:\/\/|mailto:)/i.test(href.trim());
}

const WORD = /[\p{L}\p{N}]/u;

export function parseInline(src: string): Inline[] {
  const out: Inline[] = [];
  let text = "";
  const flush = () => {
    if (text) out.push({ t: "text", v: text });
    text = "";
  };
  let i = 0;
  while (i < src.length) {
    const rest = src.slice(i);
    const prev = i > 0 ? src[i - 1]! : "";
    let m: RegExpMatchArray | null;
    if (rest[0] === "\n") {
      flush();
      out.push({ t: "br" });
      i += 1;
    } else if ((m = rest.match(/^`([^`\n]+)`/))) {
      flush();
      out.push({ t: "code", v: m[1]! });
      i += m[0].length;
    } else if ((m = rest.match(/^\[([^\]\n]+)\]\(([^)\s]+)\)/))) {
      flush();
      const children = parseInline(m[1]!);
      if (isSafeHref(m[2]!)) out.push({ t: "link", href: m[2]!, c: children });
      else out.push(...children);
      i += m[0].length;
    } else if ((m = rest.match(/^(\*\*|__)(?=\S)([\s\S]*?\S)\1/))) {
      flush();
      out.push({ t: "strong", c: parseInline(m[2]!) });
      i += m[0].length;
    } else if ((m = rest.match(/^~~(?=\S)([\s\S]*?\S)~~/))) {
      flush();
      out.push({ t: "del", c: parseInline(m[1]!) });
      i += m[0].length;
    } else if (
      (m = rest.match(/^(\*|_)(?=\S)([^\n]*?\S)\1/)) &&
      !(m[1] === "_" && WORD.test(prev)) &&
      !(m[1] === "_" && WORD.test(rest[m[0].length] ?? ""))
    ) {
      flush();
      out.push({ t: "em", c: parseInline(m[2]!) });
      i += m[0].length;
    } else if (!WORD.test(prev) && (m = rest.match(/^(https?:\/\/[^\s<>()]*[^\s<>().,;:!?'"])/))) {
      flush();
      out.push({ t: "link", href: m[1]!, c: [{ t: "text", v: m[1]! }] });
      i += m[0].length;
    } else {
      text += src[i];
      i += 1;
    }
  }
  flush();
  return out;
}

const UL = /^\s*[-*+]\s+(.*)$/;
const OL = /^\s*(\d{1,9})[.)]\s+(.*)$/;
const TASK = /^\[([ xX])\]\s+(.*)$/;

function entry(text: string): ListEntry {
  const task = text.match(TASK);
  if (task) return { checked: task[1] !== " ", c: parseInline(task[2]!) };
  return { checked: null, c: parseInline(text) };
}

export function parseMarkdown(src: string): Block[] {
  const lines = src.replace(/\r\n?/g, "\n").split("\n");
  const blocks: Block[] = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i]!;
    if (!line.trim()) {
      i += 1;
      continue;
    }
    if (/^\s*```/.test(line)) {
      const body: string[] = [];
      i += 1;
      while (i < lines.length && !/^\s*```/.test(lines[i]!)) body.push(lines[i++]!);
      i += 1;
      blocks.push({ t: "code", v: body.join("\n") });
      continue;
    }
    const heading = line.match(/^(#{1,3})\s+(.*)$/);
    if (heading) {
      blocks.push({
        t: "h",
        level: heading[1]!.length as 1 | 2 | 3,
        c: parseInline(heading[2]!.trim()),
      });
      i += 1;
      continue;
    }
    if (/^\s*([-*_])(\s*\1){2,}\s*$/.test(line)) {
      blocks.push({ t: "hr" });
      i += 1;
      continue;
    }
    if (/^\s*>/.test(line)) {
      const body: string[] = [];
      while (i < lines.length && /^\s*>/.test(lines[i]!))
        body.push(lines[i++]!.replace(/^\s*>\s?/, ""));
      blocks.push({ t: "quote", c: parseInline(body.join("\n")) });
      continue;
    }
    if (UL.test(line)) {
      const items: ListEntry[] = [];
      while (i < lines.length && UL.test(lines[i]!)) items.push(entry(lines[i++]!.match(UL)![1]!));
      blocks.push({ t: "ul", items });
      continue;
    }
    const ol = line.match(OL);
    if (ol) {
      const items: ListEntry[] = [];
      while (i < lines.length && OL.test(lines[i]!)) items.push(entry(lines[i++]!.match(OL)![2]!));
      blocks.push({ t: "ol", start: Number(ol[1]), items });
      continue;
    }
    const para: string[] = [];
    while (
      i < lines.length &&
      lines[i]!.trim() &&
      !/^\s*```/.test(lines[i]!) &&
      !/^#{1,3}\s/.test(lines[i]!) &&
      !/^\s*>/.test(lines[i]!) &&
      !UL.test(lines[i]!) &&
      !OL.test(lines[i]!)
    ) {
      para.push(lines[i++]!);
    }
    blocks.push({ t: "p", c: parseInline(para.join("\n")) });
  }
  return blocks;
}
