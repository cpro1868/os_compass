import { useEffect, useRef, useMemo } from "react";
import { marked, Renderer } from "marked";
import hljs from "highlight.js";
import DOMPurify from "dompurify";
import { openUrl } from "@tauri-apps/plugin-opener";
import "highlight.js/styles/github.css";

function resolveRelativeImageUrls(content: string, baseUrl: string): string {
  if (!baseUrl) return content;

  const githubRawBase = baseUrl
    .replace("github.com", "raw.githubusercontent.com")
    .replace("/tree/", "/");

  return content.replace(/!\[([^\]]*)\]\(([^)]+)\)/g, (match, alt, path) => {
    if (path.startsWith("http://") || path.startsWith("https://") || path.startsWith("//")) {
      return match;
    }
    const resolvedUrl = path.startsWith("/")
      ? `${githubRawBase.replace(/\/[^/]+\/[^/]+\/?$/, "")}${path}`
      : `${githubRawBase}/${path}`;
    return `![${alt}](${resolvedUrl})`;
  });
}

function createRenderer(): Renderer {
  const renderer = new Renderer();

  renderer.code = function({ text, lang }: { text: string; lang?: string }) {
    const language = lang && hljs.getLanguage(lang) ? lang : "plaintext";
    const highlighted = hljs.highlight(text, { language }).value;
    return `<pre><code class="hljs language-${language}">${highlighted}</code></pre>`;
  };

  renderer.link = function({ href, title, text }: { href: string; title?: string | null; text: string }) {
    const titleAttr = title ? ` title="${title}"` : "";
    return `<a href="${href}"${titleAttr} target="_blank" rel="noopener noreferrer">${text}</a>`;
  };

  renderer.image = function({ href, title, text }: { href: string; title?: string | null; text: string }) {
    const titleAttr = title ? ` title="${title}"` : "";
    return `<img src="${href}" alt="${text}"${titleAttr} loading="lazy" />`;
  };

  return renderer;
}

marked.setOptions({
  renderer: createRenderer(),
  breaks: true,
  gfm: true,
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
  baseUrl?: string;
}

export function MarkdownRenderer({ content, className = "", baseUrl }: MarkdownRendererProps) {
  const ref = useRef<HTMLDivElement>(null);

  const processedContent = useMemo(() => {
    if (!content) return "";
    const withResolvedUrls = baseUrl ? resolveRelativeImageUrls(content, baseUrl) : content;
    const html = marked.parse(withResolvedUrls) as string;
    return DOMPurify.sanitize(html, {
      ADD_ATTR: ["target", "class", "rel", "loading"],
      ALLOW_DATA_ATTR: false,
    });
  }, [content, baseUrl]);

  useEffect(() => {
    if (ref.current && processedContent) {
      ref.current.innerHTML = processedContent;
      ref.current.addEventListener("click", handleLinkClick);
    }

    return () => {
      if (ref.current) {
        ref.current.removeEventListener("click", handleLinkClick);
      }
    };
  }, [processedContent]);

  return (
    <div
      ref={ref}
      className={`markdown-content ${className}`}
    />
  );
}

export function renderMarkdown(content: string, baseUrl?: string): string {
  const withResolvedUrls = baseUrl ? resolveRelativeImageUrls(content, baseUrl) : content;
  const html = marked.parse(withResolvedUrls) as string;
  return DOMPurify.sanitize(html, {
    ADD_ATTR: ["target", "class", "rel", "loading"],
  });
}

export { handleLinkClick };
