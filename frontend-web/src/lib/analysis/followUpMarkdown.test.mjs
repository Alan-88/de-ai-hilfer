import test from "node:test";
import assert from "node:assert/strict";
import { renderFollowUpHtml } from "./followUpMarkdown.ts";

test("follow-up Markdown preserves paragraphs and lists while blocking active HTML and links", () => {
  const html = renderFollowUpHtml('**收据**\n\n- 已付款\n- 证明\n\n<script>alert(1)</script>\n\n[x](javascript:alert%281%29)');
  assert.match(html, /<strong>收据<\/strong>/);
  assert.match(html, /<li>已付款<\/li>/);
  assert.match(html, /&lt;script&gt;/);
  assert.doesNotMatch(html, /<script|href="javascript:/);
  assert.doesNotMatch(renderFollowUpHtml('<img src=x onerror=alert(1)>'), /<img/);
});
