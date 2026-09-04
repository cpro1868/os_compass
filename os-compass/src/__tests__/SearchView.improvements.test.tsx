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

function makeWebResults(count: number): Array<any> {
  return Array.from({ length: count }).map((_, i) => ({
    name: `Project-${i + 1}`,
    url: `https://github.com/org/repo-${i + 1}`,
    description: `desc-${i + 1}`,
    stars: 100 * (i + 1),
    forks: 10 * (i + 1),
    language: "Rust",
    match_score: 0.5,
    source: "llm",
  }));
}

describe("SearchView 三个优化项", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    searchApiMock.getSearchHistory.mockResolvedValue([]);
    searchApiMock.clearSearchHistory.mockResolvedValue(undefined);
    searchApiMock.deleteSearchHistoryItem.mockResolvedValue(undefined);
    searchApiMock.analyzeIntent.mockResolvedValue({
      intent: "clear",
      keywords: ["视频处理"],
    });
  });

  it("需求 1: web_results 超过 5 条时默认折叠，只渲染前 5 条", async () => {
    searchApiMock.intentSearch.mockResolvedValue({
      query: "视频处理",
      local_results: [],
      web_results: makeWebResults(8),
      total: 8,
      conversation_id: "c1",
    });

    render(<SearchView />);

    fireEvent.change(screen.getByPlaceholderText("search.placeholder"), {
      target: { value: "推荐视频处理" },
    });
    const sendBtn = screen.getAllByRole("button").find((b) => b.querySelector(".fa-paper-plane"))!;
    fireEvent.click(sendBtn);

    expect(await screen.findByText("Project-1")).not.toBeNull();
    expect(screen.getByText("Project-5")).not.toBeNull();
    expect(screen.queryByText("Project-6")).toBeNull();

    const expand = await screen.findByText(/search\.showAll/);
    fireEvent.click(expand);

    await waitFor(() => {
      expect(screen.queryByText("Project-8")).not.toBeNull();
    });
  });

  it("需求 2: 点击 “让大模型再推荐” 会调用后端并把推荐追加到 web_results", async () => {
    searchApiMock.intentSearch.mockResolvedValue({
      query: "视频处理",
      local_results: [],
      web_results: makeWebResults(3),
      total: 3,
      conversation_id: "c1",
    });
    searchApiMock.recommendMoreByLLM.mockResolvedValue({
      items: makeWebResults(8).map((p, i) => ({
        ...p,
        name: `More-${p.name}`,
        url: `https://github.com/org/more-repo-${i + 1}`,
        source: "llm",
      })),
    });

    render(<SearchView />);

    fireEvent.change(screen.getByPlaceholderText("search.placeholder"), {
      target: { value: "推荐视频处理" },
    });
    const sendBtn = screen.getAllByRole("button").find((b) => b.querySelector(".fa-paper-plane"))!;
    fireEvent.click(sendBtn);

    await waitFor(() => {
      expect(searchApiMock.intentSearch).toHaveBeenCalled();
    });

    const more = await screen.findByText(/search\.recommendMore/);
    fireEvent.click(more);

    await waitFor(() => {
      expect(searchApiMock.recommendMoreByLLM).toHaveBeenCalledWith("视频处理", 5);
    });
    await waitFor(() => {
      expect(screen.queryByText("More-Project-1")).not.toBeNull();
    });
  });

  it("需求 3: 点击搜索历史的 ✕ 按钮会调用 deleteSearchHistoryItem 并从列表移除", async () => {
    searchApiMock.getSearchHistory.mockResolvedValue([
      { id: 1, query: "视频剪辑", result_count: 3, created_at: "2026-09-03 18:00:00" },
      { id: 2, query: "Rust CLI", result_count: 5, created_at: "2026-09-03 18:30:00" },
    ]);

    render(<SearchView />);

    expect(await screen.findByText("视频剪辑")).not.toBeNull();
    expect(screen.getByText("Rust CLI")).not.toBeNull();

    const deleteButtons = screen.getAllByRole("button", { name: /search\.deleteHistoryItem/ });
    fireEvent.click(deleteButtons[0]);

    await waitFor(() => {
      expect(searchApiMock.deleteSearchHistoryItem).toHaveBeenCalledWith(1);
    });
    await waitFor(() => {
      expect(screen.queryByText("视频剪辑")).toBeNull();
    });
    expect(screen.queryByText("Rust CLI")).not.toBeNull();
  });
});
