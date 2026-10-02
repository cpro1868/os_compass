import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (!params) return key;
      let s = key;
      for (const [k, v] of Object.entries(params)) s = s.replace(`{{${k}}}`, String(v));
      return s;
    },
  }),
}));

const exploreApiMock = {
  getLatestExploreSession: vi.fn(),
  listExploreSessions: vi.fn(),
  getExploreMessages: vi.fn(),
  askExplore: vi.fn(),
  deleteExploreSession: vi.fn(),
  getExploreKbStatus: vi.fn(),
};
vi.mock("../api/explore", () => exploreApiMock);
vi.mock("../api/note", () => ({ noteApi: { create: vi.fn().mockResolvedValue(1) } }));
vi.mock("../stores/toastStore", () => ({ useToastStore: () => ({ showToast: vi.fn() }) }));

const { ExplorePanel } = await import("../components/ExplorePanel");

describe("ExplorePanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    exploreApiMock.getLatestExploreSession.mockResolvedValue(null);
    exploreApiMock.listExploreSessions.mockResolvedValue([]);
    exploreApiMock.getExploreKbStatus.mockResolvedValue({
      ready: true, building: false, degraded: false,
      readmeChunks: 5, hasAiReport: true, hasRunbook: true, notesCount: 2,
    });
  });

  it("发送问题后展示回答并恢复输入", async () => {
    exploreApiMock.askExplore.mockResolvedValue({
      sessionId: 10, answer: "**回答**内容", degraded: false,
      sources: [{ type: "readme_chunk", chunkIndex: 2 }],
    });
    render(<ExplorePanel projectId={1} projectName="demo" />);
    const input = screen.getByPlaceholderText("detail.explorePlaceholder");
    fireEvent.change(input, { target: { value: "做什么的？" } });
    fireEvent.keyDown(input, { key: "Enter", shiftKey: false });
    await waitFor(() => expect(screen.getByText(/回答/)).toBeTruthy());
    expect(exploreApiMock.askExplore).toHaveBeenCalledWith(1, undefined, "做什么的？");
  });

  it("加载历史会话消息", async () => {
    exploreApiMock.getLatestExploreSession.mockResolvedValue({
      id: 7, projectId: 1, title: "旧问题", createdAt: "", updatedAt: "",
    });
    exploreApiMock.getExploreMessages.mockResolvedValue([
      { id: 1, sessionId: 7, role: "user", content: "旧问题", sources: null, createdAt: "" },
      { id: 2, sessionId: 7, role: "assistant", content: "旧回答", sources: null, createdAt: "" },
    ]);
    render(<ExplorePanel projectId={1} projectName="demo" />);
    await waitFor(() => expect(screen.getByText("旧回答")).toBeTruthy());
  });

  it("LLM 报错时显示错误与重试", async () => {
    exploreApiMock.askExplore.mockRejectedValue(new Error("LLM 请求超时"));
    render(<ExplorePanel projectId={1} projectName="demo" />);
    const input = screen.getByPlaceholderText("detail.explorePlaceholder");
    fireEvent.change(input, { target: { value: "test" } });
    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() => expect(screen.getByText(/LLM 请求超时/)).toBeTruthy());
    expect(screen.getByText("detail.exploreRetry")).toBeTruthy();
  });

  it("存入笔记调用 noteApi.create 并带探索问答前缀", async () => {
    const { noteApi } = await import("../api/note");
    exploreApiMock.askExplore.mockResolvedValue({
      sessionId: 10, answer: "答案文本", degraded: false, sources: [],
    });
    render(<ExplorePanel projectId={1} projectName="demo" />);
    const input = screen.getByPlaceholderText("detail.explorePlaceholder");
    fireEvent.change(input, { target: { value: "我的问题" } });
    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() => expect(screen.getByText("答案文本")).toBeTruthy());
    fireEvent.click(screen.getByText("detail.exploreSaveNote"));
    await waitFor(() => expect(noteApi.create).toHaveBeenCalled());
    const arg = (noteApi.create as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(arg.project_id).toBe(1);
    expect(arg.content).toContain("我的问题");
    expect(arg.content).toContain("答案文本");
  });
});
