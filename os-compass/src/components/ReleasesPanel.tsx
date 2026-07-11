import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

interface DownloadUrl {
  name: string;
  url: string;
}

interface Release {
  id: string;
  project_id: number;
  tag_name: string;
  published_at: string | null;
  body: string | null;
  body_zh: string | null;
  download_urls: DownloadUrl[];
  source: string;
  fetched_at: string | null;
}

interface Project {
  id: number;
  name: string;
  url: string | null;
}

interface Toast {
  id: number;
  message: string;
  type: "success" | "error" | "info";
}

let toastId = 0;

export function ReleasesPanel({ project }: { project: Project }) {
  const { t } = useTranslation();
  const [releases, setReleases] = useState<Release[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [translating, setTranslating] = useState(false);
  const [expandedBody, setExpandedBody] = useState<Set<string>>(new Set());
  const [downloadMenuOpen, setDownloadMenuOpen] = useState<string | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);

  const showToast = (message: string, type: "success" | "error" | "info" = "info") => {
    const id = ++toastId;
    setToasts(prev => [...prev, { id, message, type }]);
    setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id));
    }, 3000);
  };

  useEffect(() => {
    loadReleases();
  }, [project.id]);

  const loadReleases = async () => {
    setLoading(true);
    try {
      const data = await invoke<Release[]>("get_releases", { projectId: project.id });
      setReleases(data);
    } catch (e) {
      console.error("Failed to load releases:", e);
    } finally {
      setLoading(false);
    }
  };

  const handleRefresh = async () => {
    setRefreshing(true);
    try {
      const data = await invoke<Release[]>("fetch_releases", { projectId: project.id });
      setReleases(data);
      if (data.length === 0) {
        showToast(t("releases.noReleasesInfo"), "info");
      } else {
        showToast(t("releases.updated"), "success");
      }
    } catch (e) {
      const error = String(e);
      if (error.includes("API_RATE_LIMITED")) {
        showToast(t("releases.rateLimited"), "error");
      } else if (error.includes("PLATFORM_NOT_SUPPORTED")) {
        showToast(t("releases.platformNotSupported"), "error");
      } else if (error.includes("NO_URL")) {
        showToast(t("releases.noUrl"), "error");
      } else {
        showToast(`${t("releases.fetchFailed")}: ${e}`, "error");
      }
    } finally {
      setRefreshing(false);
    }
  };

  const handleTranslate = async () => {
    setTranslating(true);
    try {
      for (const release of releases) {
        const text = release.body || "";
        if (text && !release.body_zh) {
          await invoke<string>("translate_release_body", {
            projectId: String(project.id),
            releaseId: release.id,
            text: text,
          });
        }
      }
      await loadReleases();
      showToast(t("releases.translateDone"), "success");
    } catch (e) {
      showToast(`${t("releases.translateFailed")}: ${e}`, "error");
    } finally {
      setTranslating(false);
    }
  };

  const handleDownload = async (url: string) => {
    try {
      await openUrl(url);
      setDownloadMenuOpen(null);
    } catch (e) {
      showToast(`${t("releases.openFailed")}: ${e}`, "error");
    }
  };

  const handleCopyLink = async (url: string) => {
    try {
      await navigator.clipboard.writeText(url);
      showToast(t("releases.linkCopied"), "success");
      setDownloadMenuOpen(null);
    } catch (e) {
      showToast(`${t("releases.copyFailed")}: ${e}`, "error");
    }
  };

  const toggleBody = (id: string) => {
    const newSet = new Set(expandedBody);
    if (newSet.has(id)) {
      newSet.delete(id);
    } else {
      newSet.add(id);
    }
    setExpandedBody(newSet);
  };

  const formatDate = (date: string | null) => {
    if (!date) return "";
    const d = new Date(date);
    return d.toLocaleDateString("zh-CN");
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin h-8 w-8 border-4 border-blue-500 border-t-transparent rounded-full"></div>
      </div>
    );
  }

  if (releases.length === 0) {
    return (
      <div className="max-w-4xl">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-bold">
            <i className="fa-solid fa-tag mr-2 text-purple-600"></i>Releases
          </h2>
          <button
            onClick={handleRefresh}
            disabled={refreshing}
            className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 flex items-center gap-2 disabled:opacity-75"
          >
            <i className={`fa-solid fa-rotate ${refreshing ? "animate-spin" : ""}`}></i>
            {t("releases.refresh")}
          </button>
        </div>
        <div className="bg-white rounded-xl border border-gray-200 p-6 text-center">
          <div className="w-16 h-16 bg-gray-100 rounded-full flex items-center justify-center mx-auto mb-4">
            <i className="fa-solid fa-inbox text-3xl text-gray-400"></i>
          </div>
          <p className="text-gray-500">{t("releases.noReleases")}</p>
          <p className="text-xs text-gray-400 mt-1">{t("releases.noReleasesHint")}</p>
        </div>
        
        {/* Toast Container */}
        {toasts.length > 0 && (
          <div className="fixed bottom-4 right-4 z-50 space-y-2">
            {toasts.map(t => (
              <div
                key={t.id}
                className={`px-4 py-3 rounded-lg shadow-lg flex items-center gap-2 animate-slide-in ${
                  t.type === "success" ? "bg-green-500 text-white" :
                  t.type === "error" ? "bg-red-500 text-white" :
                  "bg-gray-800 text-white"
                }`}
              >
                <i className={`fa-solid ${
                  t.type === "success" ? "fa-check-circle" :
                  t.type === "error" ? "fa-circle-exclamation" :
                  "fa-info-circle"
                }`}></i>
                {t.message}
              </div>
            ))}
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="max-w-4xl">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-bold">
          <i className="fa-solid fa-tag mr-2 text-purple-600"></i>Releases
        </h2>
        <div className="flex items-center gap-2">
          <button
            onClick={handleTranslate}
            disabled={translating}
            className="px-3 py-1.5 text-sm border border-gray-300 rounded-lg hover:bg-gray-50 flex items-center gap-2 disabled:opacity-75"
          >
            <i className={`fa-solid fa-language ${translating ? "animate-spin" : ""}`}></i>
            {t("releases.translate")}
          </button>
          <button
            onClick={handleRefresh}
            disabled={refreshing}
            className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 flex items-center gap-2 disabled:opacity-75"
          >
            <i className={`fa-solid fa-rotate ${refreshing ? "animate-spin" : ""}`}></i>
            {t("releases.refresh")}
          </button>
        </div>
      </div>

      <div className="bg-white rounded-xl border border-gray-200 p-6">
        <div className="space-y-4">
          {releases.map((release) => {
            const body = release.body_zh || release.body || "";
            const isExpanded = expandedBody.has(release.id);
            const isLongBody = body.length > 500;
            const displayBody = isLongBody && !isExpanded ? body.slice(0, 500) + "..." : body;

            return (
              <div key={release.id} className="border border-gray-200 rounded-lg overflow-hidden">
                <div className="bg-gray-50 px-4 py-3 flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <span className="px-2 py-1 bg-green-100 text-green-700 rounded font-mono text-sm font-medium">
                      {release.tag_name}
                    </span>
                    <span className="text-sm text-gray-500">
                      {formatDate(release.published_at)}
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    {release.download_urls.length > 0 && (
                      <div className="relative">
                        <button
                          onClick={() => setDownloadMenuOpen(downloadMenuOpen === release.id ? null : release.id)}
                          className="px-3 py-1.5 text-sm border rounded-lg hover:bg-gray-100 flex items-center gap-2"
                        >
                          <i className="fa-solid fa-download"></i>
                          {t("releases.download")}
                          <i className="fa-solid fa-chevron-down text-xs"></i>
                        </button>
                        {downloadMenuOpen === release.id && (
                          <div className="absolute right-0 mt-1 w-64 bg-white border rounded-lg shadow-lg z-10">
                            {release.download_urls.map((item, idx) => (
                              <div key={idx}>
                                <button
                                  onClick={() => handleDownload(item.url)}
                                  className="w-full px-4 py-2 text-sm hover:bg-gray-50 text-left border-b last:border-b-0"
                                >
                                  <div className="font-medium">{item.name}</div>
                                  <div className="text-xs text-gray-500 truncate">{item.url}</div>
                                </button>
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    )}
                    {release.download_urls.length > 0 && (
                      <button
                        onClick={() => handleCopyLink(release.download_urls[0].url)}
                        className="px-3 py-1.5 text-sm border rounded-lg hover:bg-gray-100"
                        title={t("releases.copyLink")}
                      >
                        <i className="fa-solid fa-copy"></i>
                      </button>
                    )}
                  </div>
                </div>
                <div className="p-4">
                  <div className="prose prose-sm max-w-none text-gray-600">
                    {isLongBody && !isExpanded ? (
                      <>
                        <div dangerouslySetInnerHTML={{ __html: displayBody.replace(/\n/g, "<br>") }} />
                        <button
                          onClick={() => toggleBody(release.id)}
                          className="text-blue-600 hover:text-blue-800 text-sm mt-2"
                        >
                          {t("releases.expandAll")}
                        </button>
                      </>
                    ) : (
                      <>
                        <div dangerouslySetInnerHTML={{ __html: displayBody.replace(/\n/g, "<br>") }} />
                        {isLongBody && (
                          <button
                            onClick={() => toggleBody(release.id)}
                            className="text-blue-600 hover:text-blue-800 text-sm mt-2"
                          >
                            {t("releases.collapse")}
                          </button>
                        )}
                      </>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>

        <div className="mt-4 pt-4 border-t border-gray-200 flex items-center justify-between text-xs text-gray-500">
          <span>
            <i className="fa-solid fa-globe mr-1"></i>
            {t("releases.dataSource")}：{releases[0]?.source === "github" ? "GitHub" : releases[0]?.source === "gitee" ? "Gitee" : releases[0]?.source}
          </span>
          <span>
            <i className="fa-solid fa-sync mr-1"></i>
            {t("releases.lastUpdate")}：{releases[0]?.fetched_at || ""}
          </span>
        </div>
      </div>
      
      {/* Toast Container */}
      {toasts.length > 0 && (
        <div className="fixed bottom-4 right-4 z-50 space-y-2">
          {toasts.map(t => (
            <div
              key={t.id}
              className={`px-4 py-3 rounded-lg shadow-lg flex items-center gap-2 ${
                t.type === "success" ? "bg-green-500 text-white" :
                t.type === "error" ? "bg-red-500 text-white" :
                "bg-gray-800 text-white"
              }`}
            >
              <i className={`fa-solid ${
                t.type === "success" ? "fa-check-circle" :
                t.type === "error" ? "fa-circle-exclamation" :
                "fa-info-circle"
              }`}></i>
              {t.message}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
