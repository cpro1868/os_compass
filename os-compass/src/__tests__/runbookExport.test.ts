import { describe, it, expect, vi } from "vitest";
import { exportRunbookAsMarkdown } from "../utils/runbookExport";

describe("exportRunbookAsMarkdown", () => {
  it("调用 export_article 并传入项目名和内容", async () => {
    const invoke = vi.fn().mockResolvedValue("C:/Users/x/Desktop/ok.md");
    const path = await exportRunbookAsMarkdown({
      invoke,
      projectName: "my-project",
      content: "# Runbook\n\nstep 1",
    });
    expect(invoke).toHaveBeenCalledWith("export_article", {
      content: "# Runbook\n\nstep 1",
      filename: "my-project-runbook",
      format: "markdown",
    });
    expect(path).toBe("C:/Users/x/Desktop/ok.md");
  });

  it("项目名包含非法字符时清洗为合法文件名", async () => {
    const invoke = vi.fn().mockResolvedValue("C:/x.md");
    await exportRunbookAsMarkdown({
      invoke,
      projectName: 'a/b\\c:d*e?f"g<h>i|j',
      content: "x",
    });
    const call = invoke.mock.calls[0][1] as { filename: string };
    expect(call.filename).toBe("a-b-c-d-e-f-g-h-i-j-runbook");
  });

  it("用户取消保存时返回 null 不抛错", async () => {
    const invoke = vi.fn().mockRejectedValue("用户取消保存");
    const result = await exportRunbookAsMarkdown({
      invoke,
      projectName: "p",
      content: "x",
    });
    expect(result).toBeNull();
  });

  it("其他错误原样抛出", async () => {
    const invoke = vi.fn().mockRejectedValue("保存文件失败: disk full");
    await expect(
      exportRunbookAsMarkdown({ invoke, projectName: "p", content: "x" })
    ).rejects.toBe("保存文件失败: disk full");
  });

  it("内容为空时直接返回 null 不调用 invoke", async () => {
    const invoke = vi.fn();
    const result = await exportRunbookAsMarkdown({
      invoke,
      projectName: "p",
      content: "",
    });
    expect(result).toBeNull();
    expect(invoke).not.toHaveBeenCalled();
  });
});
