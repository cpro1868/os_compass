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

describe("再推荐返回大模型原文", () => {
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

  it("大模型返回 raw_text 时，在新消息中直接渲染原文，不渲染选项卡", async () => {
    searchApiMock.recommendMoreByLLM.mockResolvedValue({
      items: [],
      raw_text: "为你推荐以下视频工具：\n1. FFmpeg: https://github.com/FFmpeg/FFmpeg\n2. Shotcut: 音频与视频剪辑",
    });

    render(<SearchView />);
    runSearch();
    await screen.findByText("Local-1");

    fireEvent.click(screen.getByText(/search\.recommendMore/));

    await waitFor(() => {
      expect(screen.queryByText(/为你推荐以下视频工具/)).not.toBeNull();
    });
    expect(screen.queryByText("search.recommendTitle")).toBeNull();
  });

  it("点击后先展示 loading 文案，返回后被原文替换", async () => {
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
      items: [],
      raw_text: "大模型直接返回的推荐正文",
    });

    await waitFor(() => {
      expect(screen.queryByText("大模型直接返回的推荐正文")).not.toBeNull();
    });
    expect(screen.queryByText(/search\.recommendLoading/)).toBeNull();
  });

  it("无原文且无项目时提示暂无更多推荐", async () => {
    searchApiMock.recommendMoreByLLM.mockResolvedValue({ items: [], raw_text: "" });

    render(<SearchView />);
    runSearch();
    await screen.findByText("Local-1");

    fireEvent.click(screen.getByText(/search\.recommendMore/));

    await waitFor(() => {
      expect(screen.queryByText(/search\.recommendEmpty/)).not.toBeNull();
    });
  });
});
