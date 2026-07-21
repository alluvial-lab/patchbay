import createDOMPurify, { type WindowLike } from "dompurify";
import { marked } from "marked";

export interface MarkdownRenderer {
  render(markdown: string): string;
}

/**
 * Creates a renderer bound to a browser window. Streaming message updates can
 * safely re-render their accumulated markdown through this stateless boundary.
 */
export function createMarkdownRenderer(window: Window): MarkdownRenderer {
  const purifier = createDOMPurify(window as unknown as WindowLike);

  return {
    render(markdown: string): string {
      const parsed = marked.parse(markdown, {
        async: false,
        gfm: true,
      });
      const sanitized = purifier.sanitize(parsed, {
        ALLOW_DATA_ATTR: false,
        FORBID_ATTR: ["style"],
        FORBID_TAGS: [
          "button",
          "embed",
          "form",
          "iframe",
          "input",
          "object",
          "option",
          "select",
          "style",
          "textarea",
        ],
        USE_PROFILES: { html: true },
      });
      return wrapWideContent(window.document, String(sanitized));
    },
  };
}

function wrapWideContent(document: Document, html: string): string {
  const container = document.createElement("div");
  container.className = "markdown-body";
  container.innerHTML = html;

  for (const table of container.querySelectorAll("table")) {
    if (table.parentElement?.classList.contains("markdown-table-scroll")) continue;
    const wrapper = document.createElement("div");
    wrapper.className = "markdown-table-scroll";
    wrapper.setAttribute("role", "region");
    wrapper.setAttribute("aria-label", "Scrollable table");
    table.before(wrapper);
    wrapper.append(table);
  }

  return container.outerHTML;
}
