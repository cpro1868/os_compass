import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (!params) return key;
      let s = key;
      for (const [k, v] of Object.entries(params)) {
        s = s.replace(`{{${k}}}`, String(v));
      }
      return s;
    },
  }),
}));

vi.mock("../api", () => ({
  getRecentProjects: vi.fn().mockResolvedValue([]),
}));

vi.mock("../stores/toastStore", () => ({
  useToastStore: () => ({ showToast: vi.fn() }),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

const searchApiMock = {
  intentSearch: vi.fn(),
  analyzeIntent: vi.fn(),
  getSearchHistory: vi.fn(),
  clearSearchHistory: vi.fn(),
  deleteSearchHistoryItem: vi.fn(),
  recommendMoreByLLM: vi.fn(),
};

vi.mock("../api/search", () => searchApiMock);

const { SearchView } = await import("../components/SearchView");

describe("多轮对话上下文传递与收敛验证", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    searchApiMock.getSearchHistory.mockResolvedValue([]);
  });

  it("多轮对话中第二轮会携带第一轮的用户与助手对话历史传给 analyzeIntent", async () => {
    searchApiMock.analyzeIntent.mockResolvedValueOnce({
      intent: "unclear",
      questions: ["您希望找的是库、框架还是完整应用？"],
      options: ["开发库", "完整应用"],
    });

    render(<SearchView />);

    // 第 1 轮提问
    fireEvent.change(screen.getByPlaceholderText("search.placeholder"), {
      target: { value: "给我推荐视频制作工具" },
    });
    const sendBtn = screen.getAllByRole("button").find((b) => b.querySelector(".fa-paper-plane"))!;
    fireEvent.click(sendBtn);

    // 等待助手的澄清卡片出现
    await screen.findByText("您希望找的是库、框架还是完整应用？");

    // 第 1 次调用时无前文历史
    expect(searchApiMock.analyzeIntent).toHaveBeenNthCalledWith(
      1,
      "给我推荐视频制作工具",
      []
    );

    // 第 2 轮回答：用户点击选项 "完整应用"
    searchApiMock.analyzeIntent.mockResolvedValueOnce({
      intent: "clear",
      keywords: ["视频剪辑", "完整应用", "视频处理"],
    });
    searchApiMock.intentSearch.mockResolvedValueOnce({
      query: "视频剪辑 完整应用 视频处理",
      local_results: [
        {
          name: "Shotcut",
          url: "https://github.com/mltframework/shotcut",
          description: "Cross-platform video editor",
          language: "C++",
          match_score: 0.9,
          source: "local",
        },
      ],
      web_results: [],
      total: 1,
      conversation_id: "conv-1",
    });

    fireEvent.click(screen.getByText("完整应用"));

    // 验证第 2 次调 analyzeIntent 必须携带上一轮的用户问题与助手的澄清提问
    await waitFor(() => {
      expect(searchApiMock.analyzeIntent).toHaveBeenCalledTimes(2);
    });

    const secondCallHistory = searchApiMock.analyzeIntent.mock.calls[1][1];
    expect(secondCallHistory.length).toBeGreaterThanOrEqual(1);

    const hasOriginalQuery = secondCallHistory.some(
      (m: any) => m.content.includes("给我推荐视频制作工具")
    );
    expect(hasOriginalQuery).toBe(true);

    // 验证收敛后直接进入搜索并渲染了搜索结果 Shotcut
    await screen.findByText("Shotcut");
  });
});
