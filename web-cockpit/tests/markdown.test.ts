import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { JSDOM } from "jsdom";

import { createMarkdownRenderer } from "../src/ui/markdown.js";

const representative = `# Reconnect report

A paragraph with **strong text**, [a link](https://example.com), and \`inline code\`.

- first
  - nested
- second

> Streams are delivery optimizations, not authority.

| Very long heading | Another heading | Third heading |
| --- | --- | --- |
| alpha alpha alpha alpha | beta beta beta beta | gamma gamma gamma gamma |

\`\`\`typescript
const deliberatelyLongLine = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";
\`\`\`
`;

test("representative markdown renders all required structures for a 360px container", async () => {
  const dom = new JSDOM("<!doctype html><main id='column'></main>");
  Object.defineProperty(dom.window, "innerWidth", { value: 360 });
  const renderer = createMarkdownRenderer(dom.window as unknown as Window);
  const column = dom.window.document.querySelector("#column") as HTMLElement;
  column.style.width = "360px";
  column.innerHTML = renderer.render(representative);

  assert.ok(column.querySelector("h1"));
  assert.ok(column.querySelector("ul ul"));
  assert.ok(column.querySelector("blockquote"));
  assert.ok(column.querySelector("code"));
  assert.ok(column.querySelector("pre > code"));
  assert.ok(column.querySelector(".markdown-table-scroll > table"));
  assert.equal(column.querySelector("table")!.parentElement!.getAttribute("role"), "region");

  const css = await readFile(new URL("../src/ui/markdown.css", import.meta.url), "utf8").catch(
    () => readFile(new URL("../../src/ui/markdown.css", import.meta.url), "utf8"),
  );
  assert.match(css, /\.markdown-body\s*\{[^}]*max-width:\s*100%/s);
  assert.match(css, /\.markdown-body pre\s*\{[^}]*overflow-x:\s*auto/s);
  assert.match(css, /\.markdown-table-scroll\s*\{[^}]*overflow-x:\s*auto/s);
  assert.doesNotMatch(css, /\.markdown-body pre\s*\{[^}]*overflow:\s*hidden/s);
});

test("untrusted HTML and dangerous URL/handler attributes are neutralized", () => {
  const dom = new JSDOM();
  const renderer = createMarkdownRenderer(dom.window as unknown as Window);
  const html = renderer.render(`
<script>globalThis.pwned = true</script>
[unsafe](javascript:alert(1))
<img src=x onerror="alert(2)" style="position:fixed">
<form action="https://attacker.invalid"><input name="secret"></form>
`);
  const container = dom.window.document.createElement("div");
  container.innerHTML = html;

  assert.equal(container.querySelector("script"), null);
  assert.equal(container.querySelector("form"), null);
  assert.equal(container.querySelector("input"), null);
  assert.equal(container.querySelector("[onerror]"), null);
  assert.equal(container.querySelector("[style]"), null);
  assert.equal(container.querySelector("a")?.hasAttribute("href"), false);
});

test("streaming updates are stateless re-renders of accumulated markdown", () => {
  const dom = new JSDOM();
  const renderer = createMarkdownRenderer(dom.window as unknown as Window);
  const partial = renderer.render("## Result\n\nA partial");
  const completed = renderer.render("## Result\n\nA partial response with `code`.");

  assert.match(partial, /A partial/);
  assert.match(completed, /A partial response with <code>code<\/code>/);
  assert.doesNotMatch(completed, /<script/i);
});
