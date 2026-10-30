import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { importProject } from "../api";
import { useTranslation } from "react-i18next";

type SimilarProject = {
  id: number | null;
  name: string;
  url: string | null;
  description: string | null;
  source: string;
  language: string | null;
  stars: number;
  forks: number;
  score: number | null;
  verified: boolean;
  imported: boolean;
  reason: string | null;
};

type NetworkResult = {
  source: string;
  cached_at: string | null;
  results: SimilarProject[];
  error: string | null;
};

interface SimilarPanelProps {
  projectId: number;
  onOpenProject?: (projectId: number) => void;
}

function scorePercent(score: number | null) {
  return Math.max(0, Math.min(100, Math.round((score ?? 0) * 100)));
}

function formatLanguage(raw: string | null): string {
  if (!raw) return "";
  const trimmed = raw.trim();
  if (trimmed.startsWith("[")) {
    try {
      const parsed = JSON.parse(trimmed);
      if (Array.isArray(parsed)) return parsed.filter(Boolean).slice(0, 3).join(", ");
    } catch { /* fallthrough */ }
  }
  return trimmed;
}

function formatSummary(raw: string | null, fallback: string): string {
  const summary = (raw || fallback).replace(/\s+/g, " ").trim();
  if (!summary) return fallback;
  return summary.length > 120 ? `${summary.slice(0, 120)}…` : summary;
}

export function SimilarPanel({ projectId, onOpenProject }: SimilarPanelProps) {
  const { t } = useTranslation();
  const [localResults, setLocalResults] = useState<SimilarProject[]>([]);
  const [network, setNetwork] = useState<NetworkResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [networkLoading, setNetworkLoading] = useState(false);
  const [llmLoading, setLlmLoading] = useState(false);
  const [importingUrl, setImportingUrl] = useState<string | null>(null);
  const [importedUrls, setImportedUrls] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);

  const loadLocal = useCallback(async () => {
    const results = await invoke<SimilarProject[]>("get_similar_projects", { projectId, limit: 10 });
    setLocalResults(results || []);
  }, [projectId]);

  const loadNetwork = useCallback(async (refresh = false) => {
    setNetworkLoading(true);
    try {
      const result = await invoke<NetworkResult>("get_network_similar_projects", { projectId, refresh });
      setNetwork(result);
    } catch (err) {
      setNetwork({ source: "", cached_at: null, results: [], error: String(err) });
    } finally {
      setNetworkLoading(false);
    }
  }, [projectId]);

  const loadLlmFallback = useCallback(async () => {
    setLlmLoading(true);
    setError(null);
    try {
      const results = await invoke<SimilarProject[]>("llm_similar_projects", { projectId });
      setNetwork({ source: "llm", cached_at: null, results: results || [], error: null });
    } catch (err) {
      setError(String(err));
    } finally {
      setLlmLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    Promise.all([loadLocal(), loadNetwork()])
      .catch((err) => { if (!cancelled) setError(String(err)); })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [loadLocal, loadNetwork]);

  const openProject = (url: string | null) => {
    if (url) void openUrl(url);
  };

  const importNetworkProject = async (url: string | null) => {
    if (!url) return;
    setImportingUrl(url);
    setError(null);
    try {
      const result = await importProject({ url, generate_ai_report: false, auto_translate: false });
      if (result.success || result.duplicate) {
        setImportedUrls((previous) => new Set(previous).add(url));
      } else {
        setError(result.error || t("detail.similarImportFailed"));
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setImportingUrl(null);
    }
  };

  if (loading) {
    return <div className="flex items-center justify-center h-48 text-gray-500"><i className="fa-solid fa-spinner fa-spin mr-2" />{t("detail.similarLoading")}</div>;
  }

  return (
    <div className="space-y-6">
      <section>
        <div className="flex items-center justify-between mb-3">
          <div>
            <h3 className="font-semibold text-gray-900 dark:text-gray-100">{t("detail.similarLocal")}</h3>
            <p className="text-xs text-gray-500 mt-1">{t("detail.similarLocalDesc")}</p>
          </div>
          <button onClick={() => void loadLocal()} className="text-sm text-blue-600 hover:text-blue-700"><i className="fa-solid fa-rotate mr-1" />{t("detail.refresh")}</button>
        </div>
        {localResults.some((item) => item.reason === "rule") && <div className="mb-3 rounded-lg bg-amber-50 dark:bg-amber-900/20 p-3 text-sm text-amber-700 dark:text-amber-300">{t("detail.similarRuleFallback")}</div>}
        {localResults.length === 0 ? (
          <div className="rounded-lg border border-dashed border-gray-300 dark:border-gray-600 p-6 text-center text-sm text-gray-500">{t("detail.similarNoLocal")}</div>
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
            {localResults.map((item) => {
              const percent = scorePercent(item.score);
              return <div key={`${item.id}-${item.name}`} className="rounded-lg border border-gray-200 dark:border-gray-700 p-4 bg-white dark:bg-gray-750">
                <div className="flex justify-between gap-3">
                  <button className="font-medium text-left text-blue-600 hover:underline truncate" title={item.name} onClick={() => item.id && onOpenProject ? onOpenProject(item.id) : openProject(item.url)}>{item.name}</button>
                  <span className="text-xs text-gray-500 shrink-0">{percent}%</span>
                </div>
                <div className="h-1.5 bg-gray-200 dark:bg-gray-700 rounded-full mt-2"><div className="h-1.5 bg-blue-500 rounded-full" style={{ width: `${percent}%` }} /></div>
                <p className="text-sm text-gray-600 dark:text-gray-300 mt-2 line-clamp-2" title={item.description || t("detail.noDescription")}>{formatSummary(item.description, t("detail.noDescription"))}</p>
                <div className="text-xs text-gray-500 mt-2">{item.source}{formatLanguage(item.language) ? ` · ${formatLanguage(item.language)}` : ""} · ★ {item.stars}</div>
              </div>;
            })}
          </div>
        )}
      </section>

      <section>
        <div className="flex items-center justify-between mb-3">
          <div>
            <h3 className="font-semibold text-gray-900 dark:text-gray-100">{t("detail.similarNetwork")}</h3>
            <p className="text-xs text-gray-500 mt-1">{t("detail.similarNetworkDesc")}</p>
          </div>
          <div className="flex gap-3">
            <button disabled={networkLoading} onClick={() => void loadNetwork(true)} className="text-sm text-blue-600 hover:text-blue-700 disabled:opacity-50"><i className={`fa-solid fa-rotate mr-1 ${networkLoading ? "fa-spin" : ""}`} />{t("detail.similarRefreshNetwork")}</button>
            <button disabled={llmLoading} onClick={() => void loadLlmFallback()} className="text-sm text-purple-600 hover:text-purple-700 disabled:opacity-50"><i className={`fa-solid fa-wand-magic-sparkles mr-1 ${llmLoading ? "fa-spin" : ""}`} />{t("detail.similarLlmFallback")}</button>
          </div>
        </div>
        {error || network?.error ? <div className="rounded-lg bg-red-50 dark:bg-red-900/20 p-4 text-sm text-red-600">{error || network?.error}</div> : null}
        {!networkLoading && network && network.results.length === 0 && !network.error ? <div className="rounded-lg border border-dashed border-gray-300 dark:border-gray-600 p-6 text-center text-sm text-gray-500">{t("detail.similarNoNetwork")}</div> : null}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
          {network?.results.map((item) => <div key={`${item.source}-${item.name}`} className="rounded-lg border border-gray-200 dark:border-gray-700 p-4">
            <div className="flex items-start justify-between gap-3"><div><h4 className="font-medium text-gray-900 dark:text-gray-100">{item.name}</h4><span className="text-xs text-gray-500">{item.source} · ★ {item.stars}</span></div><span className="text-xs text-gray-500">{item.verified ? t("detail.similarVerified") : t("detail.similarUnverified")}</span></div>
            <p className="text-sm text-gray-600 dark:text-gray-300 mt-2 line-clamp-2">{item.description || t("detail.noDescription")}</p>
            <div className="flex gap-2 mt-3">
              <button disabled={!item.url} onClick={() => openProject(item.url)} className="px-3 py-1.5 text-sm rounded bg-blue-600 text-white disabled:opacity-40">{t("detail.similarOpen")}</button>
              <button disabled={!item.url || importingUrl === item.url || importedUrls.has(item.url || "")} onClick={() => void importNetworkProject(item.url)} className="px-3 py-1.5 text-sm rounded border border-gray-300 dark:border-gray-600 disabled:opacity-40">
                {importedUrls.has(item.url || "") ? t("detail.similarImported") : importingUrl === item.url ? t("detail.similarImporting") : t("detail.similarImport")}
              </button>
            </div>
          </div>)}
        </div>
      </section>
    </div>
  );
}
