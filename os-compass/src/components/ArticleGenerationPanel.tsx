import { useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { marked } from "marked";
import DOMPurify from "dompurify";
import html2pdf from "html2pdf.js";

interface ArticleGenerationPanelProps {
  projectId: number;
  projectName: string;
}

type EditorMode = "source" | "preview";

export function ArticleGenerationPanel({ projectId, projectName }: ArticleGenerationPanelProps) {
  const [style, setStyle] = useState("tech_popular");
  const [wordCount, setWordCount] = useState("medium");
  const [content, setContent] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [filename, setFilename] = useState(projectName || "article");
  const [format, setFormat] = useState("markdown");
  const [editorMode, setEditorMode] = useState<EditorMode>("source");

  const htmlContent = useMemo(() => {
    if (!content) return "";
    const rawHtml = marked.parse(content) as string;
    return DOMPurify.sanitize(rawHtml, {
      ADD_TAGS: ['h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'pre', 'code', 'blockquote', 'ul', 'ol', 'li', 'p', 'strong', 'em', 'a', 'img', 'table', 'thead', 'tbody', 'tr', 'th', 'td', 'hr', 'br'],
    });
  }, [content]);

  const generatePdfHtml = (mdContent: string) => {
    const html = marked.parse(mdContent) as string;
    return `
      <!DOCTYPE html>
      <html>
      <head>
        <meta charset="UTF-8">
        <style>
          body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; padding: 40px; line-height: 1.6; }
          h1 { color: #333; margin-bottom: 20px; }
          h2, h3 { color: #444; margin-top: 24px; }
          code { background: #f5f5f5; padding: 2px 6px; border-radius: 4px; font-family: monospace; }
          pre { background: #f5f5f5; padding: 16px; border-radius: 8px; overflow-x: auto; }
          pre code { background: none; padding: 0; }
          blockquote { border-left: 4px solid #ddd; padding-left: 16px; color: #666; margin: 16px 0; }
          table { border-collapse: collapse; width: 100%; }
          th, td { border: 1px solid #ddd; padding: 8px; }
          th { background: #f5f5f5; }
        </style>
      </head>
      <body>
        ${DOMPurify.sanitize(html)}
      </body>
      </html>
    `;
  };

  const handleGenerate = async () => {
    setIsGenerating(true);
    try {
      const result = await invoke<string>("generate_article", {
        projectId,
        style,
        wordCount,
      });
      setContent(result);
      setEditorMode("preview");
      alert("文章生成成功！");
    } catch (e) {
      alert(`生成失败: ${e}`);
    } finally {
      setIsGenerating(false);
    }
  };

  const handleExportPdf = async () => {
    if (!content) {
      alert("请先生成文章内容");
      return;
    }
    try {
      const opt = {
        margin: 10,
        filename: `${filename || "article"}.pdf`,
        image: { type: "jpeg" as const, quality: 0.98 },
        html2canvas: { scale: 2 },
        jsPDF: { unit: "mm" as const, format: "a4" as const, orientation: "portrait" as const },
      };
      await html2pdf().set(opt).from(generatePdfHtml(content)).save();
      alert("PDF 导出成功！");
    } catch (e) {
      alert(`导出失败: ${e}`);
    }
  };

  const handleExport = async () => {
    if (!content) {
      alert("请先生成文章内容");
      return;
    }
    if (format === "pdf") {
      await handleExportPdf();
      return;
    }
    try {
      const path = await invoke<string>("export_article", {
        content,
        filename: filename || "article",
        format,
      });
      alert(`已保存到: ${path}`);
    } catch (e) {
      if (e !== "用户取消保存") {
        alert(`导出失败: ${e}`);
      }
    }
  };

  const wordCountOptions = [
    { value: "short", label: "短文 (500-800字)" },
    { value: "medium", label: "中等 (1000-1500字)" },
    { value: "long", label: "长文 (2000-3000字)" },
  ];

  return (
    <div className="space-y-4">
      <div className="bg-gray-50 dark:bg-gray-800 rounded-lg border p-4">
        <h3 className="font-medium mb-4 text-lg">文章生成</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium mb-1.5">文章风格</label>
            <select
              value={style}
              onChange={(e) => setStyle(e.target.value)}
              className="w-full border rounded-lg px-3 py-2 dark:bg-gray-700 dark:border-gray-600"
            >
              <option value="tech_popular">技术科普 - 通俗易懂，适合非技术背景读者</option>
              <option value="business">商业推广 - 专业严谨，突出商业价值</option>
              <option value="tech_blog">技术博客 - 技术深度与可读性并重</option>
              <option value="brief">简介说明 - 简洁明了，信息密度高</option>
              <option value="humor">幽默风趣 - 轻松愉快，适合社交媒体</option>
            </select>
          </div>
          <div>
            <label className="block text-sm font-medium mb-1.5">字数要求</label>
            <select
              value={wordCount}
              onChange={(e) => setWordCount(e.target.value)}
              className="w-full border rounded-lg px-3 py-2 dark:bg-gray-700 dark:border-gray-600"
            >
              {wordCountOptions.map((opt) => (
                <option key={opt.value} value={opt.value}>{opt.label}</option>
              ))}
            </select>
          </div>
        </div>
        <button
          onClick={handleGenerate}
          disabled={isGenerating}
          className="mt-4 px-6 py-2.5 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition flex items-center gap-2"
        >
          {isGenerating ? (
            <>
              <i className="fa-solid fa-spinner fa-spin"></i>
              生成中...
            </>
          ) : (
            <>
              <i className="fa-solid fa-wand-magic-sparkles"></i>
              生成文章
            </>
          )}
        </button>
      </div>

      <div className="bg-gray-50 dark:bg-gray-800 rounded-lg border">
        <div className="flex items-center justify-between p-3 border-b dark:border-gray-700">
          <h3 className="font-medium">文章内容</h3>
          <div className="flex items-center gap-2">
            <span className="text-sm text-gray-500 mr-2">{content.length} 字</span>
            <div className="flex rounded-lg overflow-hidden border dark:border-gray-600">
              <button
                onClick={() => setEditorMode("source")}
                className={`px-3 py-1 text-sm transition ${
                  editorMode === "source"
                    ? "bg-blue-600 text-white"
                    : "bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600"
                }`}
              >
                源码
              </button>
              <button
                onClick={() => setEditorMode("preview")}
                className={`px-3 py-1 text-sm transition ${
                  editorMode === "preview"
                    ? "bg-blue-600 text-white"
                    : "bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600"
                }`}
              >
                预览
              </button>
            </div>
          </div>
        </div>
        <div className="p-4">
          {editorMode === "source" ? (
            <textarea
              value={content}
              onChange={(e) => setContent(e.target.value)}
              rows={20}
              className="w-full p-4 font-mono text-sm border rounded-lg dark:bg-gray-900 dark:border-gray-600 resize-y min-h-[400px]"
              placeholder="生成的文章将显示在这里，您可以自由编辑..."
            />
          ) : (
            <div
              className="prose prose-slate dark:prose-invert max-w-none p-6 bg-white dark:bg-gray-900 min-h-[400px] overflow-auto rounded-lg border border-gray-200 dark:border-gray-700"
              style={{ lineHeight: '1.75' }}
            >
              <div dangerouslySetInnerHTML={{ __html: htmlContent }} />
              <style>{`
                .prose h1 { font-size: 1.875rem; font-weight: 700; margin-top: 1.5rem; margin-bottom: 1rem; color: #1e293b; border-bottom: 2px solid #e2e8f0; padding-bottom: 0.5rem; }
                .prose h2 { font-size: 1.5rem; font-weight: 600; margin-top: 1.5rem; margin-bottom: 0.75rem; color: #334155; }
                .prose h3 { font-size: 1.25rem; font-weight: 600; margin-top: 1.25rem; margin-bottom: 0.5rem; color: #475569; }
                .prose h4 { font-size: 1.125rem; font-weight: 600; margin-top: 1rem; margin-bottom: 0.5rem; }
                .prose p { margin-bottom: 1rem; }
                .prose blockquote { border-left: 4px solid #3b82f6; padding-left: 1rem; margin: 1rem 0; color: #64748b; font-style: italic; background: #f8fafc; padding: 1rem; border-radius: 0.5rem; }
                .prose ul, .prose ol { margin: 1rem 0; padding-left: 1.5rem; }
                .prose li { margin-bottom: 0.5rem; }
                .prose ul { list-style-type: disc; }
                .prose ol { list-style-type: decimal; }
                .prose code { background: #f1f5f9; padding: 0.125rem 0.375rem; border-radius: 0.25rem; font-size: 0.875rem; color: #dc2626; }
                .prose pre { background: #1e293b; color: #e2e8f0; padding: 1rem; border-radius: 0.5rem; overflow-x: auto; margin: 1rem 0; }
                .prose pre code { background: transparent; color: inherit; padding: 0; }
                .prose a { color: #3b82f6; text-decoration: underline; }
                .prose a:hover { color: #2563eb; }
                .prose hr { border: none; border-top: 1px solid #e2e8f0; margin: 2rem 0; }
                .prose table { width: 100%; border-collapse: collapse; margin: 1rem 0; }
                .prose th, .prose td { border: 1px solid #e2e8f0; padding: 0.75rem; text-align: left; }
                .prose th { background: #f8fafc; font-weight: 600; }
                .prose img { max-width: 100%; height: auto; border-radius: 0.5rem; margin: 1rem 0; }
                .prose strong { font-weight: 600; color: #1e293b; }
                .prose em { font-style: italic; }
                .dark .prose h1 { color: #f1f5f9; border-bottom-color: #334155; }
                .dark .prose h2 { color: #e2e8f0; }
                .dark .prose h3 { color: #cbd5e1; }
                .dark .prose blockquote { background: #1e293b; color: #94a3b8; border-color: #3b82f6; }
                .dark .prose code { background: #334155; color: #fb923c; }
                .dark .prose pre { background: #0f172a; }
                .dark .prose pre code { background: transparent; }
                .dark .prose th { background: #1e293b; }
                .dark .prose th, .dark .prose td { border-color: #334155; }
              `}</style>
            </div>
          )}
        </div>
      </div>

      <div className="bg-gray-50 dark:bg-gray-800 rounded-lg border p-4">
        <div className="flex items-center gap-4 flex-wrap">
          <input
            type="text"
            value={filename}
            onChange={(e) => setFilename(e.target.value)}
            className="px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 w-48"
            placeholder="文件名"
          />
          <select
            value={format}
            onChange={(e) => setFormat(e.target.value)}
            className="px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600"
          >
            <option value="markdown">Markdown (.md)</option>
            <option value="txt">文本 (.txt)</option>
            <option value="pdf">PDF (.pdf)</option>
          </select>
          <button
            onClick={handleExport}
            disabled={!content}
            className="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition flex items-center gap-2"
          >
            <i className="fa-solid fa-download"></i>
            导出
          </button>
        </div>
      </div>
    </div>
  );
}
