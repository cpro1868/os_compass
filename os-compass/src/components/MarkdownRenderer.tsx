import { useEffect, useRef } from "react";
import { marked } from "marked";
import hljs from "highlight.js";
import DOMPurify from "dompurify";
import { openUrl } from "@tauri-apps/plugin-opener";
import "highlight.js/styles/github.css";

const renderer = new marked.Renderer();
renderer.code = function({ text, lang }: { text: string; lang?: string }) {
  const language = lang && hljs.getLanguage(lang) ? lang : "plaintext";
  const highlighted = hljs.highlight(text, { language }).value;
  return `<pre><code class="hljs language-${language}">${highlighted}</code></pre>`;
};

marked.setOptions({
  renderer,
});

async function handleLinkClick(e: MouseEvent) {
  const target = e.target as HTMLElement;
  const anchor = target.closest("a") as HTMLAnchorElement | null;
  if (!anchor) return;
  const href = anchor.getAttribute("href");
  if (!href) return;
  if (href.startsWith("#") || href.startsWith("javascript:")) return;
  e.preventDefault();
  e.stopPropagation();
  try {
    await openUrl(href);
  } catch (err) {
    console.error("Failed to open URL:", err);
  }
}

interface MarkdownRendererProps {
  content: string;
  className?: string;
}

export function MarkdownRenderer({ content, className = "" }: MarkdownRendererProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (ref.current && content) {
      const html = marked.parse(content) as string;
      const sanitized = DOMPurify.sanitize(html, {
        ADD_ATTR: ["target", "class"],
      });
      ref.current.innerHTML = sanitized;

      ref.current.querySelectorAll("a").forEach((a) => {
        a.setAttribute("target", "_blank");
        a.setAttribute("rel", "noopener noreferrer");
      });

      ref.current.addEventListener("click", handleLinkClick);
    }

    return () => {
      if (ref.current) {
        ref.current.removeEventListener("click", handleLinkClick);
      }
    };
  }, [content]);

  return (
    <div
      ref={ref}
      className={`markdown-content ${className}`}
    />
  );
}

export function renderMarkdown(content: string): string {
  const html = marked.parse(content) as string;
  const sanitized = DOMPurify.sanitize(html, {
    ADD_ATTR: ["target", "class"],
  });
  return `<div class="markdown-content">${sanitized}</div>`;
}

export { handleLinkClick };
