import { describe, expect, test } from "vitest";
import { parseInline, parseMarkdown } from "./markdown";

describe("markdown", () => {
  test("inline styles", () => {
    expect(parseInline("**bold** and _it_ and `x`")).toEqual([
      { t: "strong", c: [{ t: "text", v: "bold" }] },
      { t: "text", v: " and " },
      { t: "em", c: [{ t: "text", v: "it" }] },
      { t: "text", v: " and " },
      { t: "code", v: "x" },
    ]);
  });

  test("snake_case stays text", () => {
    expect(parseInline("my_var_name")).toEqual([{ t: "text", v: "my_var_name" }]);
  });

  test("unsafe links lose their target", () => {
    expect(parseInline("[x](javascript:void)")).toEqual([{ t: "text", v: "x" }]);
    expect(parseInline("[x](https://a.b)")[0]).toMatchObject({ t: "link", href: "https://a.b" });
  });

  test("blocks", () => {
    const blocks = parseMarkdown("# Title\n\n- [x] done\n- open\n\n1. one\n\n```\ncode\n```\ntext");
    expect(blocks.map((b) => b.t)).toEqual(["h", "ul", "ol", "code", "p"]);
    expect(blocks[1]).toMatchObject({ items: [{ checked: true }, { checked: null }] });
  });
});
