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

describe("意图搜索 - 用户验收失败：再推荐 5 个无反应", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    searchApiMock.getSearchHistory.mockResolvedValue([]);
    searchApiMock.analyzeIntent.mockResolvedValue({ intent: "clear", keywords: ["地图工具"] });
  });

  /**
   * 用户报告：
   *   1. 搜索 "地图工具" → 本地 6 条（驴唇不对马嘴）
   *   2. 点击 "让大模型再推荐 5 个" → 没有任何反应
   *
   * 后端实调研证：LLM 30 秒返回 5 个完美匹配。
   * 所以问题一定在前后端衔接处。
   */
  it("场景A：6 条本地 + 后端返回 5 个 LLM 推荐时，5 个 LLM 卡片必须可见", async () => {
    const llmItems = [
      { name: "Leaflet", url: "https://github.com/Leaflet/Leaflet", description: "JS map library", language: "JavaScript", match_score: 0.7, source: "llm_recommend" },
      { name: "Mapbox", url: "https://github.com/mapbox/mapbox-gl-js", description: "WebGL vector maps", language: "JavaScript", match_score: 0.7, source: "llm_recommend" },
      { name: "OpenLayers", url: "https://github.com/openlayers/openlayers", description: "GIS web maps", language: "JavaScript", match_score: 0.7, source: "llm_recommend" },
      { name: "MapLibre", url: "https://github.com/maplibre/maplibre-gl-js", description: "Fork of Mapbox GL JS", language: "JavaScript", match_score: 0.7, source: "llm_recommend" },
      { name: "QGIS", url: "https://github.com/qgis/QGIS", description: "Desktop GIS", language: "C++", match_score: 0.7, source: "llm_recommend" },
    ];

    searchApiMock.intentSearch.mockResolvedValue({
      query: "地图工具",
      local_results: Array.from({ length: 6 }).map((_, i) => ({
        name: `Local-${i + 1}`,
        url: `https://github.com/org/local-${i + 1}`,
        description: `desc ${i + 1}`,
        language: "Rust",
        project_id: 100 + i,
        match_score: 0.7,
        source: "local",
      })),
      web_results: [],
      total: 6,
      conversation_id: "c1",
    });
    searchApiMock.recommendMoreByLLM.mockResolvedValue({ items: llmItems });

    render(<SearchView />);

    fireEvent.change(screen.getByPlaceholderText("search.placeholder"), {
      target: { value: "地图工具" },
    });
    const sendBtn = screen.getAllByRole("button").find((b) => b.querySelector(".fa-paper-plane"))!;
    fireEvent.click(sendBtn);

    // 等待初始 6 条本地结果出现
    await screen.findByText("Local-1");

    // 触发"再推荐 5 个"
    const askMoreBtn = await screen.findByText(/search\.recommendMore/);
    fireEvent.click(askMoreBtn);

    // 等后端返回后，5 个 LLM 卡片必须出现
    await waitFor(() => {
      expect(screen.getByText("Leaflet")).not.toBeNull();
    }, { timeout: 5000 });

    // 5 个 LLM 卡片全部出现
    expect(screen.getByText("Leaflet")).not.toBeNull();
    expect(screen.getByText("Mapbox")).not.toBeNull();
    expect(screen.getByText("OpenLayers")).not.toBeNull();
    expect(screen.getByText("MapLibre")).not.toBeNull();
    expect(screen.getByText("QGIS")).not.toBeNull();

    // LLM联网 计数应 ≥ 5
    const llmOnlineBadge = screen.getAllByText(/LLM联网|search\.llmOnline/i);
    expect(llmOnlineBadge.length).toBeGreaterThan(0);
  });
});
