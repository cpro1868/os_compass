import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { EmptyState } from "./EmptyState";
import { ErrorState } from "./ErrorState";

interface StatsResponse {
  total_projects: number;
  status_distribution: { status: string; count: number }[];
  health_distribution: { label: string; min: number; max: number; count: number }[];
  category_distribution: { category_id: number | null; category_name: string; count: number }[];
  language_distribution: { language: string; count: number }[];
  tag_distribution: { tag_id: number; tag_name: string; count: number }[];
}

const STATUS_LABELS: Record<string, string> = {
  TO_EXPLORE: "待探索",
  DIVING: "深度研究中",
  IN_USE: "已落地/在用",
  ABANDONED: "弃用/避坑",
};

const STATUS_COLORS: Record<string, string> = {
  TO_EXPLORE: "bg-blue-500",
  DIVING: "bg-yellow-500",
  IN_USE: "bg-green-500",
  ABANDONED: "bg-gray-500",
};

const HEALTH_COLORS = ["bg-green-500", "bg-blue-500", "bg-yellow-500", "bg-red-500"];

interface StatsViewProps {
  onBack: () => void;
}

export function StatsView({ onBack }: StatsViewProps) {
  const { t } = useTranslation();
  const [stats, setStats] = useState<StatsResponse | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<StatsResponse>("get_stats")
      .then(setStats)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center">
          <div className="inline-block animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600 dark:border-blue-400 mb-2"></div>
          <p className="text-sm text-gray-500 dark:text-gray-400">{t("stats.loading")}</p>
        </div>
      </div>
    );
  }

  if (!stats) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <ErrorState
          title={t("stats.loadFailed")}
          message={t("stats.loadFailedDesc")}
          onRetry={() => window.location.reload()}
        />
      </div>
    );
  }

  if (stats.total_projects === 0) {
    return (
      <div className="flex flex-col h-full">
        <header className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
          <div className="flex items-center gap-4">
            <button onClick={onBack} className="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300">
              <i className="fa-solid fa-arrow-left mr-2"></i>{t("stats.return")}
            </button>
            <h1 className="text-xl font-bold dark:text-gray-100">{t("stats.title")}</h1>
          </div>
        </header>
        <div className="flex-1 flex items-center justify-center">
          <EmptyState
            icon="fa-chart-pie"
            title={t("stats.emptyTitle")}
            description={t("stats.emptyDesc")}
            actionLabel={t("stats.emptyAction")}
            onAction={() => window.dispatchEvent(new CustomEvent("openImport"))}
          />
        </div>
      </div>
    );
  }

  const maxStatusCount = Math.max(...stats.status_distribution.map((s) => s.count), 1);
  const maxCategoryCount = Math.max(...stats.category_distribution.map((c) => c.count), 1);
  const maxLangCount = Math.max(...stats.language_distribution.map((l) => l.count), 1);
  const maxTagCount = Math.max(...stats.tag_distribution.map((t) => t.count), 1);
  const maxHealthCount = Math.max(...stats.health_distribution.map((h) => h.count), 1);

  return (
    <div className="flex flex-col h-full">
      <header className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
        <div className="flex items-center gap-4">
          <button onClick={onBack} className="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300">
            <i className="fa-solid fa-arrow-left mr-2"></i>{t("stats.return")}
          </button>
          <h1 className="text-xl font-bold dark:text-gray-100">{t("stats.title")}</h1>
          <span className="text-sm text-gray-500 dark:text-gray-400">{t("stats.totalProjects", { count: stats.total_projects })}</span>
        </div>
      </header>

      <div className="flex-1 overflow-auto p-6">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {/* 状态分布 */}
          <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
            <h2 className="text-lg font-bold mb-4">
              <i className="fa-solid fa-chart-bar mr-2 text-blue-600 dark:text-blue-400"></i>{t("stats.statusDist")}
            </h2>
            <div className="space-y-3">
              {stats.status_distribution.map((item) => (
                <div key={item.status}>
                  <div className="flex justify-between text-sm mb-1">
                    <span>{STATUS_LABELS[item.status] || item.status}</span>
                    <span className="text-gray-500 dark:text-gray-400">{item.count}</span>
                  </div>
                  <div className="w-full bg-gray-100 dark:bg-gray-700 rounded-full h-3">
                    <div
                      className={`${STATUS_COLORS[item.status] || "bg-gray-400 dark:bg-gray-500"} h-3 rounded-full transition-all`}
                      style={{ width: `${(item.count / maxStatusCount) * 100}%` }}
                    />
                  </div>
                </div>
              ))}
              {stats.status_distribution.length === 0 && (
                <p className="text-gray-400 dark:text-gray-500 text-sm">{t("stats.noData")}</p>
              )}
            </div>
          </div>

          {/* 健康度分布 */}
          <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
            <h2 className="text-lg font-bold mb-4">
              <i className="fa-solid fa-heart-pulse mr-2 text-green-600 dark:text-green-400"></i>{t("stats.healthDist")}
            </h2>
            <div className="space-y-3">
              {stats.health_distribution.map((item, i) => (
                <div key={item.label}>
                  <div className="flex justify-between text-sm mb-1">
                    <span>{item.label}</span>
                    <span className="text-gray-500 dark:text-gray-400">{item.count}</span>
                  </div>
                  <div className="w-full bg-gray-100 dark:bg-gray-700 rounded-full h-3">
                    <div
                      className={`${HEALTH_COLORS[i] || "bg-gray-400 dark:bg-gray-500"} h-3 rounded-full transition-all`}
                      style={{ width: `${(item.count / maxHealthCount) * 100}%` }}
                    />
                  </div>
                </div>
              ))}
              {stats.health_distribution.length === 0 && (
                <p className="text-gray-400 dark:text-gray-500 text-sm">{t("stats.noRatingData")}</p>
              )}
            </div>
          </div>

          {/* 分类分布 */}
          <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
            <h2 className="text-lg font-bold mb-4">
              <i className="fa-solid fa-folder-tree mr-2 text-indigo-600"></i>{t("stats.categoryDist")}
            </h2>
            <div className="space-y-3">
              {stats.category_distribution.map((item) => (
                <div key={item.category_id ?? "none"}>
                  <div className="flex justify-between text-sm mb-1">
                    <span className="truncate">{item.category_name}</span>
                    <span className="text-gray-500 dark:text-gray-400">{item.count}</span>
                  </div>
                  <div className="w-full bg-gray-100 dark:bg-gray-700 rounded-full h-3">
                    <div
                      className="bg-indigo-500 h-3 rounded-full transition-all"
                      style={{ width: `${(item.count / maxCategoryCount) * 100}%` }}
                    />
                  </div>
                </div>
              ))}
              {stats.category_distribution.length === 0 && (
                <p className="text-gray-400 dark:text-gray-500 text-sm">{t("stats.noData")}</p>
              )}
            </div>
          </div>

          {/* 语言分布 */}
          <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
            <h2 className="text-lg font-bold mb-4">
              <i className="fa-solid fa-code mr-2 text-orange-600"></i>{t("stats.languageDist")}
            </h2>
            <div className="space-y-2">
              {stats.language_distribution.map((item) => (
                <div key={item.language} className="flex items-center gap-3">
                  <span className="text-sm w-32 truncate">{item.language}</span>
                  <div className="flex-1 bg-gray-100 dark:bg-gray-700 rounded-full h-2.5">
                    <div
                      className="bg-orange-500 h-2.5 rounded-full transition-all"
                      style={{ width: `${(item.count / maxLangCount) * 100}%` }}
                    />
                  </div>
                  <span className="text-xs text-gray-500 dark:text-gray-400 w-8 text-right">{item.count}</span>
                </div>
              ))}
              {stats.language_distribution.length === 0 && (
                <p className="text-gray-400 dark:text-gray-500 text-sm">{t("stats.noData")}</p>
              )}
            </div>
          </div>

          {/* 标签分布 */}
          <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6 lg:col-span-2">
            <h2 className="text-lg font-bold mb-4">
              <i className="fa-solid fa-tags mr-2 text-purple-600"></i>{t("stats.tagDist")}
            </h2>
            <div className="flex flex-wrap gap-2">
              {stats.tag_distribution.map((item) => {
                const size = 0.8 + (item.count / maxTagCount) * 0.6;
                return (
                  <span
                    key={item.tag_id}
                    className="px-3 py-1.5 bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300 rounded-full"
                    style={{ fontSize: `${size}rem` }}
                  >
                    {item.tag_name} <span className="text-purple-400 dark:text-purple-500">({item.count})</span>
                  </span>
                );
              })}
              {stats.tag_distribution.length === 0 && (
                <p className="text-gray-400 dark:text-gray-500 text-sm">{t("stats.noTagData")}</p>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
