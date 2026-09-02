import { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";
import {
  intentSearch,
  analyzeIntent,
  getSearchHistory,
  type ProjectMatch,
  type SearchResult,
  type IntentAnalysis,
  type SearchHistoryItem,
} from "../api/search";
import { getRecentProjects } from "../api";
import type { Project } from "../types";
import { useToastStore } from "../stores/toastStore";

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
  const [clarifyInput, setClarifyInput] = useState("");
  const [results, setResults] = useState<ProjectMatch[]>([]);
  const [llmText, setLlmText] = useState<string | undefined>();
  const [recommendation, setRecommendation] = useState<SearchResult["recommendation"]>();
  const [clarification, setClarification] = useState<IntentAnalysis | null>(null);
  const [recentProjects, setRecentProjects] = useState<Project[]>([]);
  const [history, setHistory] = useState<SearchHistoryItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);
  const conversationIdRef = useRef<string | null>(null);

  const intentCache = useRef<Map<string, { data: IntentAnalysis; expiresAt: number }>>(new Map());
  const searchCache = useRef<Map<string, { data: SearchResult; expiresAt: number }>>(new Map());

  const generateCacheKey = (text: string): string => {
    const normalized = text.toLowerCase().trim();
    let hash = 0;
    for (let i = 0; i < normalized.length; i++) {
      const char = normalized.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash;
    }
    return hash.toString(16);
  };

  const getIntentFromCache = (key: string): IntentAnalysis | null => {
    const cached = intentCache.current.get(key);
    if (cached && cached.expiresAt > Date.now()) return cached.data;
    if (cached) intentCache.current.delete(key);
    return null;
  };

  const setIntentToCache = (key: string, data: IntentAnalysis): void => {
    intentCache.current.set(key, { data, expiresAt: Date.now() + 2 * 60 * 60 * 1000 });
  };

  const getSearchFromCache = (key: string): SearchResult | null => {
    const cached = searchCache.current.get(key);
    if (cached && cached.expiresAt > Date.now()) return cached.data;
    if (cached) searchCache.current.delete(key);
    return null;
  };

  const setSearchToCache = (key: string, data: SearchResult): void => {
    searchCache.current.set(key, { data, expiresAt: Date.now() + 30 * 60 * 1000 });
  };

  const loadRecentProjects = useCallback(async () => {
    try {
      const projects = await getRecentProjects(20);
      setRecentProjects(projects);
    } catch {
      console.error("Failed to load recent projects");
    }
  }, []);

  const loadHistory = useCallback(async () => {
    try {
      const data = await getSearchHistory(20);
      setHistory(data);
    } catch {
      console.error("Failed to load search history");
    }
  }, []);

  useEffect(() => {
    loadRecentProjects();
    loadHistory();
  }, [loadRecentProjects, loadHistory]);

  useEffect(() => {
    const handleVaultChanged = () => {
      intentCache.current.clear();
      searchCache.current.clear();
      conversationIdRef.current = null;
      setResults([]);
      setLlmText(undefined);
      setRecommendation(undefined);
      setClarification(null);
      setHasSearched(false);
      loadHistory();
      loadRecentProjects();
    };
    window.addEventListener("vault-changed", handleVaultChanged);
    return () => window.removeEventListener("vault-changed", handleVaultChanged);
  }, [loadHistory, loadRecentProjects]);

  function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("timeout")), timeoutMs);
      promise
        .then(result => { clearTimeout(timer); resolve(result); })
        .catch(err => { clearTimeout(timer); reject(err); });
    });
  }

  const runSearch = useCallback(async (rawQuery: string) => {
    const trimmed = rawQuery.trim();
    if (!trimmed || loading) return;
    setLoading(true);
    setHasSearched(true);
    setClarification(null);
    setClarifyInput("");
    setQuery("");
    try {
      const cacheKey = generateCacheKey(trimmed);
      let intent = getIntentFromCache(cacheKey);
      if (!intent) {
        try {
          intent = await withTimeout(analyzeIntent(trimmed), 30000);
          setIntentToCache(cacheKey, intent);
        } catch {
          intent = { intent: "clear", keywords: [trimmed] };
        }
      }

      if (intent.intent === "unclear" && intent.options && intent.options.length > 0) {
        setClarification(intent);
        setHasSearched(true);
        return;
      }

      const searchQuery = intent.keywords?.join(" ") || trimmed;
      const searchCacheKey = generateCacheKey(searchQuery);
      let result = getSearchFromCache(searchCacheKey);

      if (!result) {
        try {
          const currentConvId = conversationIdRef.current || undefined;
          result = await withTimeout(intentSearch(searchQuery, currentConvId), 60000);
          setSearchToCache(searchCacheKey, result);
        } catch {
          result = { query: searchQuery, local_results: [], web_results: [], total: 0, conversation_id: "" };
        }
      }

      if (!conversationIdRef.current && result.conversation_id) {
        conversationIdRef.current = result.conversation_id;
      }

      setResults([...(result.local_results || []), ...(result.web_results || [])]);
      setLlmText(result.llm_text);
      setRecommendation(result.recommendation);
      loadHistory();
    } catch {
      showToast(t("search.error.searchFailed") || "搜索失败", "error");
      setResults([]);
      setLlmText(undefined);
      setRecommendation(undefined);
    } finally {
      setLoading(false);
    }
  }, [loading, showToast, t, loadHistory]);

  const handleSearch = useCallback(() => {
    runSearch(query);
  }, [query, runSearch]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      handleSearch();
    }
  };

  const handleProjectClick = async (projectUrl: string | null) => {
    if (!projectUrl) return;
    try {
      await invoke("open_project_detail", { url: projectUrl });
    } catch {
      showToast(t("search.error.openFailed") || "打开项目详情失败", "error");
    }
  };

  const quickSuggestions = [
    "Vue.js 相关项目",
    "Python 机器学习",
    "Rust CLI 工具",
    "Java 微服务框架",
  ];

  const ProjectCard = (props: { item: ProjectMatch }) => {
    const { item } = props;
    const platform = getPlatformIcon(item.url);
    const activity = getActivityLevel(item.stars);
    const isLocal = item.source === "local";
    return (
      <div
        onClick={() => handleProjectClick(item.url)}
        className="bg-gray-50 dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4 hover:border-blue-500/50 cursor-pointer transition"
      >
        <div className="flex items-start gap-4">
          <div className={`w-12 h-12 ${platform.includes("github") ? "bg-blue-600/20" : platform.includes("python") ? "bg-orange-600/20" : "bg-purple-600/20"} rounded-xl flex items-center justify-center flex-shrink-0`}>
            <i className={`${platform} text-xl ${platform.includes("github") ? "text-blue-500" : platform.includes("python") ? "text-orange-500" : "text-purple-500"}`}></i>
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1 flex-wrap">
              <h3 className="font-semibold text-gray-900 dark:text-white truncate">
                {item.name || "未知项目"}
              </h3>
              <span className={`flex items-center gap-1 px-2 py-0.5 rounded text-xs border ${
                isLocal
                  ? "bg-blue-100 dark:bg-blue-900/40 text-blue-600 dark:text-blue-400 border-blue-200 dark:border-blue-700"
                  : "bg-purple-100 dark:bg-purple-900/40 text-purple-600 dark:text-purple-400 border-purple-200 dark:border-purple-700"
              }`}>
                <i className={`fa-solid ${isLocal ? "fa-database" : "fa-wand-magic-sparkles"} text-[10px]`}></i>
                {isLocal ? t("search.source.local") : t("search.source.llm")}
              </span>
              {item.health_score !== undefined && (
                <span className={`px-2 py-0.5 ${activity.color} bg-opacity-20 text-xs rounded text-gray-300`}>
                  ❤️ {item.health_score}
                </span>
              )}
            </div>
            <p className="text-sm text-gray-500 dark:text-gray-400 line-clamp-2">
              {item.description || "暂无描述"}
            </p>
            <div className="flex items-center gap-4 mt-2 text-xs text-gray-400">
              {item.stars !== undefined && (
                <span className="flex items-center gap-1">
                  <i className="fa-solid fa-star text-yellow-500"></i>
                  {item.stars.toLocaleString()}
                </span>
              )}
              {item.forks !== undefined && (
                <span className="flex items-center gap-1">
                  <i className="fa-solid fa-code-branch"></i>
                  {item.forks.toLocaleString()}
                </span>
              )}
              {item.language && (
                <span className="flex items-center gap-1">
                  <i className="fa-solid fa-code"></i>
                  {item.language}
                </span>
              )}
            </div>
          </div>
          <span className="flex items-center gap-1 px-3 py-1.5 bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400 rounded-lg text-xs flex-shrink-0">
            <i className="fa-solid fa-check"></i>
            {t("search.inLibrary") || "已入库"}
          </span>
        </div>
      </div>
    );
  };

  return (
    <div className="flex h-full bg-white dark:bg-gray-900">
      {/* 左侧搜索区域 */}
      <main className="flex-1 flex flex-col overflow-hidden">
        {/* 搜索结果区域（可滚动） */}
        <div className="flex-1 overflow-y-auto px-6 py-6">
          <div className="max-w-3xl mx-auto">
            {!hasSearched && !loading && !clarification && (
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

            {clarification && !loading && (
              <div className="bg-white dark:bg-gray-800 rounded-2xl border border-gray-200 dark:border-gray-700 p-5 mb-4">
                <p className="text-gray-700 dark:text-gray-300 mb-4">
                  {t("search.clarification.prompt") || "请选择或补充您的需求："}
                </p>
                <div className="flex flex-wrap gap-2 mb-4">
                  {(clarification.options || []).map((option, idx) => (
                    <button
                      key={idx}
                      onClick={() => runSearch(option)}
                      className="px-3 py-1.5 text-sm bg-gray-100 dark:bg-gray-700 hover:bg-blue-100 dark:hover:bg-blue-900/30 border border-gray-200 dark:border-gray-600 hover:border-blue-500 rounded-lg transition flex items-center gap-1.5"
                    >
                      <i className="fa-solid fa-hand-pointer text-blue-500"></i>
                      {option}
                    </button>
                  ))}
                </div>
                <div className="flex items-center gap-2">
                  <input
                    type="text"
                    value={clarifyInput}
                    onChange={(e) => setClarifyInput(e.target.value)}
                    placeholder={t("search.clarification.placeholder") || "补充您的具体需求..."}
                    className="flex-1 px-3 py-2 text-sm bg-gray-50 dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded-lg focus:outline-none focus:border-blue-500 text-gray-900 dark:text-white"
                    onKeyDown={(e) => {
                      if (e.key === "Enter" && clarifyInput.trim()) {
                        runSearch(clarifyInput);
                      }
                    }}
                  />
                  <button
                    onClick={() => runSearch(clarifyInput)}
                    disabled={!clarifyInput.trim() || loading}
                    className="px-4 py-2 bg-blue-600 hover:bg-blue-500 disabled:bg-gray-300 dark:disabled:bg-gray-600 rounded-xl text-sm text-white transition flex items-center gap-2"
                  >
                    <i className="fa-solid fa-paper-plane"></i>
                    <span>搜索</span>
                  </button>
                </div>
              </div>
            )}

            {hasSearched && !loading && !clarification && results.length > 0 && (
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
                  {results.map((result, index) => (
                    <ProjectCard key={index} item={result} />
                  ))}
                </div>
                {recommendation && (
                  <div className="mt-4 p-4 bg-gray-50 dark:bg-gray-700/50 rounded-xl border border-gray-200 dark:border-gray-700">
                    <h4 className="font-semibold text-sm text-gray-700 dark:text-gray-300 mb-3 flex items-center gap-2">
                      <i className="fa-solid fa-wand-magic-sparkles text-blue-500"></i>
                      {t("search.smartRecommendation") || "智能建议"}
                    </h4>
                    {recommendation.categories.length > 0 && (
                      <div className="mb-3">
                        <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">{t("search.relatedCategories") || "相关分类"}</p>
                        <div className="flex flex-wrap gap-1.5">
                          {recommendation.categories.map((cat, idx) => (
                            <span key={idx} className="px-2 py-0.5 bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 text-xs rounded-md">
                              {cat}
                            </span>
                          ))}
                        </div>
                      </div>
                    )}
                    {recommendation.tags.length > 0 && (
                      <div className="mb-3">
                        <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">{t("search.relatedTags") || "关联标签"}</p>
                        <div className="flex flex-wrap gap-1.5">
                          {recommendation.tags.map((tag, idx) => (
                            <span key={idx} className="px-2 py-0.5 bg-gray-200 dark:bg-gray-600 text-gray-600 dark:text-gray-300 text-xs rounded-md">
                              {tag}
                            </span>
                          ))}
                        </div>
                      </div>
                    )}
                    {recommendation.suggestions.length > 0 && (
                      <div>
                        <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">{t("search.expandedSearch") || "扩展搜索"}</p>
                        <div className="flex flex-wrap gap-1.5">
                          {recommendation.suggestions.map((suggestion, idx) => (
                            <button
                              key={idx}
                              onClick={() => runSearch(suggestion)}
                              className="px-2 py-0.5 bg-purple-100 dark:bg-purple-900/30 text-purple-600 dark:text-purple-400 text-xs rounded-md hover:bg-purple-200 dark:hover:bg-purple-900/50 transition"
                            >
                              {suggestion}
                            </button>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                )}
                {llmText && (
                  <div className="mt-4 p-4 bg-purple-50 dark:bg-purple-900/20 rounded-xl border border-purple-200 dark:border-purple-800">
                    <div className="prose prose-sm dark:prose-invert max-w-none">
                      <ReactMarkdown>{llmText}</ReactMarkdown>
                    </div>
                  </div>
                )}
              </div>
            )}

            {hasSearched && !loading && !clarification && results.length === 0 && (
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

      {/* 右侧侧栏：搜索历史 + 最近使用 */}
      <aside className="w-72 border-l border-gray-200 dark:border-gray-800 p-4 bg-gray-50 dark:bg-gray-900/50 overflow-y-auto">
        <div className="mb-6">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-sm font-medium text-gray-600 dark:text-gray-400 flex items-center gap-2">
              <i className="fa-solid fa-clock-rotate-left"></i>
              {t("search.history") || "搜索历史"}
            </h3>
          </div>
          {history.length === 0 ? (
            <p className="text-xs text-gray-400 dark:text-gray-500 text-center py-4">
              {t("search.noHistory") || "暂无搜索历史"}
            </p>
          ) : (
            <div className="space-y-1">
              {history.slice(0, 10).map((item) => (
                <button
                  key={item.id}
                  onClick={() => runSearch(item.query)}
                  className="w-full text-left px-3 py-2 text-sm rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 transition flex items-center gap-2 truncate"
                >
                  <i className="fa-regular fa-message text-gray-400 text-xs"></i>
                  <span className="truncate">{item.query}</span>
                </button>
              ))}
            </div>
          )}
        </div>

        <div>
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
                  onClick={() => handleProjectClick(project.url)}
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
        </div>
      </aside>
    </div>
  );
}
