import DOMPurify from "dompurify";

const ALLOWED_TAGS = [
  "p",
  "br",
  "hr",
  "strong",
  "em",
  "b",
  "i",
  "u",
  "s",
  "ul",
  "ol",
  "li",
  "h1",
  "h2",
  "h3",
  "h4",
  "h5",
  "h6",
  "blockquote",
  "code",
  "pre",
  "table",
  "thead",
  "tbody",
  "tr",
  "th",
  "td",
  "a",
  "img",
  "span",
  "figure",
  "figcaption",
];

export function sanitizeShowNotes(html: string): string {
  return DOMPurify.sanitize(html, {
    ALLOWED_TAGS,
    ALLOWED_ATTR: ["href", "src", "alt", "title"],
    ALLOWED_URI_REGEXP: /^https?:/i,
    ALLOW_DATA_ATTR: false,
    KEEP_CONTENT: true,
    RETURN_TRUSTED_TYPE: false,
  });
}

DOMPurify.addHook("uponSanitizeAttribute", (node, eventData) => {
  const data = eventData as unknown as { attr: string; attrValue: string };
  if (node instanceof HTMLImageElement && data.attr === "src") {
    const value = node.getAttribute("src");
    if (value?.toLowerCase().startsWith("http://")) {
      data.attrValue = value.replace(/^http:\/\//i, "https://");
    }
  }
});
