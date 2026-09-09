import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SearchView } from "../components/SearchView";
import * as searchApi from "../api/search";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("../api", () => ({
  getRecentProjects: vi.fn().mockResolvedValue([]),
}));

vi.mock("../stores/toastStore", () => ({
  useToastStore: () => ({
    showToast: vi.fn(),
  }),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("SearchView 组件渲染端到端测试", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("搜索完成后应退出 TypingIndicator 并渲染搜索结果项目卡片", async () => {
    vi.spyOn(searchApi, "getSearchHistory").mockResolvedValue([]);
    vi.spyOn(searchApi, "analyzeIntent").mockResolvedValue({
      intent: "clear",
      keywords: ["视频处理"],
    });
    vi.spyOn(searchApi, "intentSearch").mockResolvedValue({
      query: "视频处理",
      local_results: [],
      web_results: [
        {
          name: "FFmpeg/FFmpeg",
          url: "https://github.com/FFmpeg/FFmpeg",
          description: "超强大的音视频处理库",
          stars: 46000,
          forks: 12000,
          language: "C",
          match_score: 0.9,
          source: "llm",
        },
      ],
      total: 1,
      conversation_id: "conv_test_123",
      recommendation: {
        categories: ["音视频处理"],
        tags: ["C", "FFmpeg"],
        suggestions: ["FFmpeg 替代品"],
      },
    });

    render(<SearchView />);

    const textarea = screen.getByPlaceholderText("search.placeholder");
    fireEvent.change(textarea, { target: { value: "推荐视频处理工具" } });

    const buttons = screen.getAllByRole("button");
    fireEvent.click(buttons.find((button) => button.querySelector(".fa-paper-plane"))!);

    await waitFor(() => {
      expect(searchApi.analyzeIntent).toHaveBeenCalledWith("推荐视频处理工具", expect.any(Array));
    });
    await waitFor(() => {
      expect(searchApi.intentSearch).toHaveBeenCalledWith("视频处理", undefined);
    });

    await waitFor(() => {
      expect(screen.queryByText(/正在分析语义|正在搜索|search\.searching/)).toBeNull();
    }, { timeout: 3000 });

    screen.debug();

    expect(screen.queryByText("FFmpeg/FFmpeg")).not.toBeNull();
    expect(screen.queryByText("超强大的音视频处理库")).not.toBeNull();
  });
});
