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

const { invoke } = await import("@tauri-apps/api/core");
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

describe("意图搜索结果卡片三项修复", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    searchApiMock.getSearchHistory.mockResolvedValue([]);
    searchApiMock.analyzeIntent.mockResolvedValue({ intent: "clear", keywords: ["视频处理"] });
  });

  it("卡片不显示原始多语言 JSON 串，只显示首个语言", async () => {
    searchApiMock.intentSearch.mockResolvedValue({
      query: "视频处理",
      local_results: [
        {
          name: "shotcut",
          url: "https://github.com/mltcrop/shotcut",
          description: "video editor",
          language: '["C++","QML","JavaScript"]',
          stars: 8000,
          health_score: 90,
          project_id: 23,
          match_score: 0.7,
          source: "local",
        },
      ],
      web_results: [],
      total: 1,
      conversation_id: "c1",
    });

    render(<SearchView />);
    runSearch();

    await screen.findByText("shotcut");
    expect(screen.getByText("C++")).not.toBeNull();
    expect(screen.queryByText(/"C\+\+"/)).toBeNull();
    expect(screen.queryByText("search.inLibrary")).toBeNull();
    expect(screen.queryByText("search.source.local")).toBeNull();
  });

  it("点击本地项目卡片派发 openProjectDetail 事件而不是调用不存在的命令", async () => {
    searchApiMock.intentSearch.mockResolvedValue({
      query: "视频处理",
      local_results: makeLocal(1),
      web_results: [],
      total: 1,
      conversation_id: "c1",
    });

    const seen: Array<number> = [];
    const listener = (e: Event) => {
      seen.push((e as CustomEvent).detail.projectId as number);
    };
    window.addEventListener("openProjectDetail", listener);

    render(<SearchView />);
    runSearch();

    fireEvent.click(await screen.findByText("Local-1"));

    await waitFor(() => {
      expect(seen).toEqual([100]);
    });
    expect(invoke).not.toHaveBeenCalledWith("open_project_detail", expect.anything());

    window.removeEventListener("openProjectDetail", listener);
  });

  it("点击非本地项目卡片用系统浏览器打开仓库地址", async () => {
    searchApiMock.intentSearch.mockResolvedValue({
      query: "视频处理",
      local_results: [],
      web_results: [
        {
          name: "FFmpeg",
          url: "https://github.com/FFmpeg/FFmpeg",
          description: "media framework",
          language: "C",
          match_score: 0.6,
          source: "llm",
        },
      ],
      total: 1,
      conversation_id: "c1",
    });

    render(<SearchView />);
    runSearch();

    fireEvent.click(await screen.findByText("FFmpeg"));

    await waitFor(() => {
      expect(openUrlMock).toHaveBeenCalledWith("https://github.com/FFmpeg/FFmpeg");
    });
  });

  it("点击“让大模型再推荐”后新结果立即可见，不被折叠隐藏", async () => {
    searchApiMock.intentSearch.mockResolvedValue({
      query: "视频处理",
      local_results: makeLocal(6),
      web_results: [],
      total: 6,
      conversation_id: "c1",
    });
    searchApiMock.recommendMoreByLLM.mockResolvedValue({
      items: Array.from({ length: 5 }).map((_, i) => ({
        name: `More-${i + 1}`,
        url: `https://github.com/org/more-${i + 1}`,
        description: `more desc ${i + 1}`,
        language: "Python",
        match_score: 0.6,
        source: "llm",
      })),
    });

    render(<SearchView />);
    runSearch();

    const more = await screen.findByText(/search\.recommendMore/);
    fireEvent.click(more);

    await waitFor(() => {
      expect(searchApiMock.recommendMoreByLLM).toHaveBeenCalledWith("视频处理", 5);
    });
    await waitFor(() => {
      expect(screen.queryByText("More-1")).not.toBeNull();
    });
    expect(screen.queryByText("More-5")).not.toBeNull();
  });

  it("再推荐返回 0 条时新聊天消息给出提示而不是静默", async () => {
    searchApiMock.intentSearch.mockResolvedValue({
      query: "视频处理",
      local_results: makeLocal(2),
      web_results: [],
      total: 2,
      conversation_id: "c1",
    });
    searchApiMock.recommendMoreByLLM.mockResolvedValue({ items: [] });

    render(<SearchView />);
    runSearch();

    const more = await screen.findByText(/search\.recommendMore/);
    fireEvent.click(more);

    await waitFor(() => {
      expect(searchApiMock.recommendMoreByLLM).toHaveBeenCalled();
    });
    await waitFor(() => {
      expect(screen.queryByText(/search\.recommendEmpty/)).not.toBeNull();
    });
  });
});
