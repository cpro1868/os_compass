import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import ReactMarkdown from "react-markdown";
import { intentSearch } from "../api/search";
import { getRecentProjects } from "../api";
import type { Project } from "../types";
import { useToastStore } from "../stores/toastStore";
import type { ProjectMatch } from "../api/search";

interface HomeSearchResult extends ProjectMatch {
  project_name?: string;
  project_url?: string;
}

function getPlatformIcon(url: string | null): string {
  if (!url) return "fa-solid fa-link";
  const u = url.toLowerCase();
  if (u.includes("github")) return "fa-brands fa-github";
  if (u.includes("gitee")) return "fa-solid fa-code-branch";
  if (u.includes("gitlab")) return "fa-brands fa-gitlab";
  if (u.includes("npm")) return "fa-brands fa-npm";
  if (u.includes("pypi") || u.includes("python")) return "fa-brands fa-python";
  if (u.includes("crates")) return "fa-solid fa-cube";
  return "fa-solid fa-link";
}

function getActivityLevel(stars?: number): { label: string; color: string } {
  if (!stars) return { label: "活跃度未知", color: "bg-slate-600" };
  if (stars > 50000) return { label: "活跃度极高", color: "bg-green-600" };
  if (stars > 10000) return { label: "活跃度高", color: "bg-green-600/70" };
  if (stars > 1000) return { label: "活跃度一般", color: "bg-yellow-600" };
  return { label: "活跃度低", color: "bg-slate-600" };
}

export function HomePage() {
  const { t } = useTranslation();
  const { showToast } = useToastStore();
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<HomeSearchResult[]>([]);
  const [llmText, setLlmText] = useState<string | undefined>();
  const [recentProjects, setRecentProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);

  useEffect(() => {
    loadRecentProjects();
  }, []);

  const loadRecentProjects = async () => {
    try {
      const projects = await getRecentProjects(20);
      setRecentProjects(projects);
    } catch {
      console.error("Failed to load recent projects");
    }
  };

  const handleSearch = useCallback(async () => {
    if (!query.trim()) return;
    setLoading(true);
    setHasSearched(true);
    try {
      const data = await intentSearch(query);
      setResults(data.local_results || []);
      setLlmText(data.llm_text);
    } catch {
      showToast(t("search.error.searchFailed") || "搜索失败", "error");
      setResults([]);
      setLlmText(undefined);
    } finally {
      setLoading(false);
    }
  }, [query, showToast, t]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      handleSearch();
    }
  };

  const handleProjectClick = (project: Project) => {
    window.dispatchEvent(new CustomEvent("openProjectDetail", { detail: { projectId: project.id } }));
  };

  const handleResultClick = (url: string | null) => {
    if (!url) return;
    const project = recentProjects.find(p => p.url === url);
    if (project) {
      handleProjectClick(project);
    } else {
      showToast("项目详情功能开发中", "info");
    }
  };

  const quickSuggestions = [
    "Vue.js 相关项目",
    "Python 机器学习",
    "Rust CLI 工具",
    "Java 微服务框架",
  ];

  return (
    <div className="flex h-full bg-white dark:bg-gray-900">
      {/* 左侧搜索区域 */}
      <main className="flex-1 flex flex-col overflow-hidden">
        {/* 搜索结果区域（可滚动） */}
        <div className="flex-1 overflow-y-auto px-6 py-6">
          <div className="max-w-3xl mx-auto">
            {!hasSearched && !loading && (
              <div className="text-center py-12">
                <div className="inline-flex items-center justify-center w-16 h-16 bg-gradient-to-br from-blue-500 to-indigo-600 rounded-2xl mb-4 shadow-lg">
                  <i className="fa-solid fa-compass text-3xl text-white"></i>
                </div>
                <h1 className="text-2xl font-bold mb-2 text-gray-900 dark:text-white">
                  {t("home.title") || "开源罗盘"}
                </h1>
                <p className="text-gray-500 dark:text-gray-400">
                  {t("home.subtitle") || "AI 开源资产智能管家"}
                </p>
              </div>
            )}

            {loading && (
              <div className="flex items-center justify-center py-12">
                <div className="text-center">
                  <div className="inline-block animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600 mb-4"></div>
                  <p className="text-gray-500 dark:text-gray-400">{t("search.searching")}</p>
                </div>
              </div>
            )}

            {hasSearched && !loading && results.length > 0 && (
              <div>
                <div className="flex items-center gap-2 mb-4">
                  <div className="w-6 h-6 bg-green-600/20 rounded-lg flex items-center justify-center">
                    <i className="fa-solid fa-check text-green-400 text-xs"></i>
                  </div>
                  <span className="text-sm text-gray-500 dark:text-gray-400">
                    找到 {results.length} 个相关项目
                  </span>
                </div>
                <div className="space-y-3">
                  {results.map((result, index) => {
                    const platform = getPlatformIcon(result.url);
                    const activity = getActivityLevel(result.stars);
                    return (
                      <div
                        key={index}
                        onClick={() => handleResultClick(result.url)}
                        className="bg-gray-50 dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4 hover:border-blue-500/50 cursor-pointer transition"
                      >
                        <div className="flex items-start gap-4">
                          <div className={`w-12 h-12 ${platform.includes("github") ? "bg-blue-600/20" : platform.includes("python") ? "bg-orange-600/20" : "bg-purple-600/20"} rounded-xl flex items-center justify-center flex-shrink-0`}>
                            <i className={`${platform} text-xl ${platform.includes("github") ? "text-blue-500" : platform.includes("python") ? "text-orange-500" : "text-purple-500"}`}></i>
                          </div>
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2 mb-1">
                              <h3 className="font-semibold text-gray-900 dark:text-white truncate">
                                {result.name || "未知项目"}
                              </h3>
                              <span className={`px-2 py-0.5 ${activity.color} bg-opacity-20 text-xs rounded text-gray-300`}>
                                {activity.label}
                              </span>
                            </div>
                            <p className="text-sm text-gray-500 dark:text-gray-400 line-clamp-2">
                              {result.description || "暂无描述"}
                            </p>
                            <div className="flex items-center gap-4 mt-2 text-xs text-gray-400">
                              {result.stars !== undefined && (
                                <span className="flex items-center gap-1">
                                  <i className="fa-solid fa-star text-yellow-500"></i>
                                  {result.stars.toLocaleString()}
                                </span>
                              )}
                              {result.forks !== undefined && (
                                <span className="flex items-center gap-1">
                                  <i className="fa-solid fa-code-branch"></i>
                                  {result.forks.toLocaleString()}
                                </span>
                              )}
                              {result.language && (
                                <span className="flex items-center gap-1">
                                  <i className="fa-solid fa-code"></i>
                                  {result.language}
                                </span>
                              )}
                            </div>
                          </div>
                          <button className="px-4 py-2 bg-blue-600/20 hover:bg-blue-600/30 text-blue-400 rounded-lg text-sm transition flex-shrink-0">
                            查看详情 →
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>
                {llmText && (
                  <div className="mt-4 p-4 bg-purple-50 dark:bg-purple-900/20 rounded-xl border border-purple-200 dark:border-purple-800">
                    <div className="prose prose-sm dark:prose-invert max-w-none">
                      <ReactMarkdown>{llmText}</ReactMarkdown>
                    </div>
                  </div>
                )}
              </div>
            )}

            {hasSearched && !loading && results.length === 0 && (
              <div className="text-center py-12">
                <div className="w-16 h-16 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center mx-auto mb-4">
                  <i className="fa-solid fa-search text-2xl text-gray-400"></i>
                </div>
                <p className="text-gray-500 dark:text-gray-400">未找到相关项目</p>
                <p className="text-sm text-gray-400 dark:text-gray-500 mt-1">尝试其他关键词</p>
              </div>
            )}
          </div>
        </div>

        {/* 搜索输入区（底部固定） */}
        <div className="border-t border-gray-200 dark:border-gray-800 px-6 py-4 bg-white dark:bg-gray-900 flex-shrink-0">
          <div className="max-w-3xl mx-auto">
            {/* 快捷提示 */}
            <div className="flex flex-wrap gap-2 mb-3">
              <span className="text-xs text-gray-400">快捷：</span>
              {quickSuggestions.map((suggestion, i) => (
                <button
                  key={i}
                  onClick={() => setQuery(suggestion)}
                  className="px-3 py-1 bg-gray-100 hover:bg-gray-200 dark:bg-gray-800 dark:hover:bg-gray-700 rounded-full text-xs text-gray-500 dark:text-gray-400 transition"
                >
                  {suggestion}
                </button>
              ))}
            </div>
            {/* 搜索框 */}
            <div className="bg-gray-100 dark:bg-gray-800 rounded-2xl border border-gray-200 dark:border-gray-700 p-3">
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 bg-blue-600/20 rounded-xl flex items-center justify-center flex-shrink-0">
                  <i className="fa-solid fa-robot text-blue-400 text-sm"></i>
                </div>
                <input
                  type="text"
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder={t("home.placeholder") || "粘贴开源项目链接..."}
                  className="flex-1 bg-transparent text-sm py-2 focus:outline-none placeholder-gray-400 text-gray-900 dark:text-white"
                />
                <button
                  onClick={handleSearch}
                  disabled={!query.trim() || loading}
                  className="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:bg-gray-300 dark:disabled:bg-gray-600 rounded-xl font-medium transition flex items-center gap-2 text-sm text-white"
                >
                  <i className="fa-solid fa-paper-plane"></i>
                  <span>搜索</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </main>

      {/* 右侧最近使用侧栏 */}
      <aside className="w-72 border-l border-gray-200 dark:border-gray-800 p-4 bg-gray-50 dark:bg-gray-900/50 overflow-y-auto">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-medium text-gray-600 dark:text-gray-400">最近使用</h3>
          <button className="text-xs text-blue-500 hover:text-blue-400 transition">
            查看全部 →
          </button>
        </div>
        <div className="space-y-3">
          {recentProjects.slice(0, 10).map((project) => {
            const platform = getPlatformIcon(project.url);
            const statusMap: Record<string, string> = {
              "TO_EXPLORE": "待探索",
              "DIVING": "深入了解",
              "IN_USE": "使用中",
              "ABANDONED": "已弃用",
            };
            return (
              <div
                key={project.id}
                onClick={() => handleProjectClick(project)}
                className="bg-white dark:bg-gray-800 rounded-xl p-3 border border-gray-200 dark:border-gray-700 hover:border-blue-500/50 cursor-pointer transition"
              >
                <div className="flex items-center gap-2 mb-2">
                  <div className={`w-8 h-8 ${platform.includes("github") ? "bg-blue-600/20" : "bg-slate-600/20"} rounded-lg flex items-center justify-center`}>
                    <i className={`${platform} text-sm ${platform.includes("github") ? "text-blue-500" : "text-gray-400"}`}></i>
                  </div>
                  <span className="text-xs px-2 py-0.5 bg-gray-100 dark:bg-gray-700 rounded text-gray-500 dark:text-gray-400">
                    {statusMap[project.lifecycle_status] || project.lifecycle_status}
                  </span>
                </div>
                <h4 className="font-medium text-sm text-gray-900 dark:text-white truncate">
                  {project.name}
                </h4>
                <p className="text-xs text-gray-500 mt-0.5 flex items-center gap-2">
                  <i className="fa-solid fa-star text-yellow-500"></i>
                  {project.stars?.toLocaleString() || 0}
                </p>
              </div>
            );
          })}
          {recentProjects.length === 0 && (
            <div className="text-center py-8">
              <i className="fa-solid fa-inbox text-2xl text-gray-300 dark:text-gray-600 mb-2"></i>
              <p className="text-xs text-gray-400">暂无最近使用</p>
            </div>
          )}
        </div>
      </aside>
    </div>
  );
}
