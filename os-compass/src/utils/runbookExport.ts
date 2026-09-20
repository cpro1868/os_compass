/**
 * 将 Runbook 内容导出为 Markdown 文件。
 *
 * - 用户通过系统"另存为"对话框选择目录并修改文件名（默认 `<项目名>-runbook.md`）。
 * - 用户取消保存时返回 `null`，不视为错误。
 * - 其他错误原样抛出，由调用方负责提示。
 */

interface Invoke {
  <T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
}

export interface ExportRunbookOptions {
  invoke: Invoke;
  projectName: string;
  content: string;
}

/** Windows 文件名非法字符替换为 `-` */
export function sanitizeFilename(name: string): string {
  return name.replace(/[\\/:*?"<>|]/g, "-").trim();
}

export async function exportRunbookAsMarkdown(
  opts: ExportRunbookOptions
): Promise<string | null> {
  const { invoke, projectName, content } = opts;
  if (!content || !content.trim()) {
    return null;
  }
  const filename = `${sanitizeFilename(projectName || "runbook")}-runbook`;
  try {
    const path = await invoke<string>("export_article", {
      content,
      filename,
      format: "markdown",
    });
    return path;
  } catch (e) {
    if (e === "用户取消保存") {
      return null;
    }
    throw e;
  }
}
