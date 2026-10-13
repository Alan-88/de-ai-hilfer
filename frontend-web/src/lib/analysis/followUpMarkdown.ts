import { marked, Renderer } from "marked";

function escapeHtml(text: string): string {
  return text.replace(/[&<>"']/g, char => ({"&":"&amp;", "<":"&lt;", ">":"&gt;", '"':"&quot;", "'":"&#39;"}[char]!));
}

export function renderFollowUpHtml(markdown: string): string {
  const renderer = new Renderer();
  renderer.html = ({text}) => escapeHtml(text);
  renderer.link = ({href, tokens}) => {
    const label = renderer.parser.parseInline(tokens);
    try {
      const url = new URL(href, "https://de.alan-studio.uk");
      if (!["http:", "https:", "mailto:"].includes(url.protocol)) return label;
      return `<a href="${escapeHtml(url.href)}" rel="noopener noreferrer">${label}</a>`;
    } catch { return label; }
  };
  renderer.image = ({text}) => escapeHtml(text);
  return marked.parse(markdown, {renderer}) as string;
}
