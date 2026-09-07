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

const showToastMock = vi.fn();
vi.mock("../stores/toastStore", () => ({
  useToastStore: () => ({ showToast: (...args: unknown[]) => showToastMock(...args) }),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const openUrlMock = vi.fn();
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: (...args: unknown[]) => openUrlMock(...args),
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

function makeLocal(count: number): Array<any> {
  return Array.from({ length: count }).map((_, i) => ({
    name: `Local-${i + 1}`,
    url: `https://github.com/org/local-${i + 1}`,
    description: `local desc ${i + 1}`,
    language: "Rust",
    project_id: 100 + i,
    match_score: 0.7,
    source: "local",
  }));
}

async function runSearch() {
  fireEvent.change(screen.getByPlaceholderText("search.placeholder"), {
    target: { value: "推荐视频处理" },
  });
  const sendBtn = screen.getAllByRole("button").find((b) => b.querySelector(".fa-paper-plane"))!;
  fireEvent.click(sendBtn);
}

describe("再推荐以新聊天消息返回", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    searchApiMock.getSearchHistory.mockResolvedValue([]);
    searchApiMock.analyzeIntent.mockResolvedValue({ intent: "clear", keywords: ["视频处理"] });
    searchApiMock.intentSearch.mockResolvedValue({
      query: "视频处理",
      local_results: makeLocal(2),
      web_results: [],
      total: 2,
      conversation_id: "c1",
    });
  });

  it("点击后新开一条聊天消息显示 loading，返回后在新消息中展示推荐项目", async () => {
    let resolveRecommend: (v: unknown) => void = () => {};
    searchApiMock.recommendMoreByLLM.mockReturnValue(
      new Promise((resolve) => {
        resolveRecommend = resolve;
      })
    );

    render(<SearchView />);
    runSearch();
    await screen.findByText("Local-1");

    fireEvent.click(screen.getByText(/search\.recommendMore/));

    expect(await screen.findByText(/search\.recommendLoading/)).not.toBeNull();

    resolveRecommend({
      items: [
        { name: "New-1", url: "https://github.com/org/new-1", description: "n1", language: "Go", match_score: 0.6 },
      ],
    });
    await waitFor(() => {
      expect(screen.queryByText("New-1")).not.toBeNull();
    });
    expect(screen.queryByText(/search\.recommendTitle/)).not.toBeNull();
    expect(screen.queryByText(/search\.recommendLoading/)).toBeNull();
  });

  it("LLM 返回 0 条时新消息提示暂无推荐", async () => {
    searchApiMock.recommendMoreByLLM.mockResolvedValue({ items: [] });

    render(<SearchView />);
    runSearch();
    await screen.findByText("Local-1");

    fireEvent.click(screen.getByText(/search\.recommendMore/));

    await waitFor(() => {
      expect(screen.queryByText(/search\.recommendEmpty/)).not.toBeNull();
    });
  });

  it("LLM 调用失败时新消息提示失败", async () => {
    searchApiMock.recommendMoreByLLM.mockRejectedValue(new Error("timeout"));

    render(<SearchView />);
    runSearch();
    await screen.findByText("Local-1");

    fireEvent.click(screen.getByText(/search\.recommendMore/));

    await waitFor(() => {
      expect(screen.queryByText(/search\.recommendFailed/)).not.toBeNull();
    });
  });

  it("推荐结果不追加到原消息的卡片列表（原消息结果数不变）", async () => {
    searchApiMock.recommendMoreByLLM.mockResolvedValue({
      items: [
        { name: "Fresh-1", url: "https://github.com/org/fresh-1", description: "f1", language: "Go", match_score: 0.6 },
      ],
    });

    render(<SearchView />);
    runSearch();
    await screen.findByText("Local-1");

    const llmCountBefore = screen.getAllByText(/search\.llmOnline/).length;
    fireEvent.click(screen.getByText(/search\.recommendMore/));

    await waitFor(() => {
      expect(screen.queryByText("Fresh-1")).not.toBeNull();
    });
    expect(screen.getAllByText(/search\.llmOnline/).length).toBe(llmCountBefore);
  });
});
