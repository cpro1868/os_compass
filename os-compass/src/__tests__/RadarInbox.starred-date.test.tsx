import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

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

const showToastMock = vi.fn();
vi.mock("../stores/toastStore", () => ({
  useToastStore: () => ({ showToast: (...args: unknown[]) => showToastMock(...args) }),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

const mockItems = [
  {
    id: 1,
    title: "Old Starred Project",
    project_name: "old-starred",
    url: "https://github.com/a/old-starred",
    description: "An item starred long ago",
    status: "collected",
    source_id: 1,
    published_at: "2025-01-01T00:00:00Z",
    fetched_at: "2025-01-01T00:00:00Z",
  },
  {
    id: 2,
    title: "Recent Unread Project",
    project_name: "recent-unread",
    url: "https://github.com/b/recent-unread",
    description: "An unread item recently fetched",
    status: "unread",
    source_id: 1,
    published_at: new Date().toISOString(),
    fetched_at: new Date().toISOString(),
  },
];

const radarApiMock = {
  getRadarItems: vi.fn().mockResolvedValue({
    items: mockItems,
    total: 2,
    total_pages: 1,
    page: 1,
    page_size: 20,
  }),
  getRadarSources: vi.fn().mockResolvedValue([]),
  getSupportedPlatformDomains: vi.fn().mockResolvedValue([]),
  getRadarSchedule: vi.fn().mockResolvedValue({ enabled: false }),
};

vi.mock("../api/radar", () => radarApiMock);

const { RadarInbox } = await import("../components/RadarInbox");

describe("情报雷达收藏页与常规流日期隔离与筛选表现", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("切换到收藏页时日期自动清空，展示全部历史收藏（不被 7 天限制过滤）", async () => {
    render(<RadarInbox />);

    await screen.findByText("recent-unread");

    // 获取日期输入框
    const dateInputs = document.querySelectorAll('input[type="date"]') as NodeListOf<HTMLInputElement>;
    expect(dateInputs.length).toBe(2);
    // 全部 Tab 下默认有起止日期
    expect(dateInputs[0].value).not.toBe("");
    expect(dateInputs[1].value).not.toBe("");

    // 点击切换到“收藏”Tab（key 为 collected）
    const starredTabBtn = screen.getByRole("button", { name: /radar\.tabs\.collected|收藏/i });
    fireEvent.click(starredTabBtn);

    // 收藏 Tab 下起止日期自动清空为无限制
    expect(dateInputs[0].value).toBe("");
    expect(dateInputs[1].value).toBe("");

    // 历史收藏项目不受 7 天过滤阻断，正常展示
    expect(await screen.findByText("old-starred")).not.toBeNull();
  });

  it("从收藏页切回全部/未读时，严格还原其原有的日期限制（不影响常规流的时效性）", async () => {
    render(<RadarInbox />);
    await screen.findByText("recent-unread");

    const dateInputs = document.querySelectorAll('input[type="date"]') as NodeListOf<HTMLInputElement>;
    const defaultStart = dateInputs[0].value;
    const defaultEnd = dateInputs[1].value;

    // 切换到收藏
    const starredTabBtn = screen.getByRole("button", { name: /radar\.tabs\.collected|收藏/i });
    fireEvent.click(starredTabBtn);
    expect(dateInputs[0].value).toBe("");
    expect(dateInputs[1].value).toBe("");

    // 切回“未读”Tab
    const unreadTabBtn = screen.getByRole("button", { name: /radar\.tabs\.unread|未读/i });
    fireEvent.click(unreadTabBtn);

    // 验证未读 Tab 严格恢复了原有日期限制
    expect(dateInputs[0].value).toBe(defaultStart);
    expect(dateInputs[1].value).toBe(defaultEnd);
  });
});
