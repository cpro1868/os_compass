import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { importProject, getCategories } from "../api";
import { invoke } from "@tauri-apps/api/core";
import { ManualAddDialog } from "./ManualAddDialog";
import type { Category } from "../types";

interface ImportModalProps {
  onClose: () => void;
  onSuccess: () => void;
}

interface ImportResult {
  url: string;
  success: boolean;
  error?: string;
  name?: string;
  duplicate?: { projectId: number; projectName: string };
}

export function ImportModal({ onClose, onSuccess }: ImportModalProps) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<"single" | "batch">("single");
  const [urls, setUrls] = useState("");
  const [singleUrl, setSingleUrl] = useState("");
  const [categoryId, setCategoryId] = useState<number | null>(null);
  const [categories, setCategories] = useState<Category[]>([]);
  const [generateAIReport, setGenerateAIReport] = useState(true);
  const [autoTranslate, setAutoTranslate] = useState(true);
  const [importing, setImporting] = useState(false);
  const [results, setResults] = useState<ImportResult[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [showManualAdd, setShowManualAdd] = useState(false);
  const [currentStep, setCurrentStep] = useState("");

  useEffect(() => {
    getCategories().then(setCategories).catch(() => {});
  }, []);

  const activeUrl = mode === "single" ? singleUrl : urls;
  const urlList = activeUrl.split("\n").filter((u) => u.trim());

  const handleImport = async () => {
    if (urlList.length === 0) return;

    setImporting(true);
    setResults([]);
    setCurrentIndex(0);
    setError(null);

    const importResults: ImportResult[] = [];
    const successProjectIds: number[] = [];
    let settingsLang = "zh-CN";

    try {
      const settings = await invoke<{ defaultLanguage: string }>("get_settings");
      settingsLang = settings?.defaultLanguage || "zh-CN";
    } catch {}

    for (let i = 0; i < urlList.length; i++) {
      const url = urlList[i].trim();
      setCurrentIndex(i + 1);

      try {
        setCurrentStep(t("import.fetchingInfo"));
        const result = await importProject({ 
          url,
          category_id: categoryId ?? undefined,
          generate_ai_report: generateAIReport,
        });

        if (result.duplicate) {
          importResults.push({
            url,
            success: false,
            error: t("import.duplicate", { name: result.duplicate.project_name }),
            duplicate: { projectId: result.duplicate.project_id, projectName: result.duplicate.project_name },
          });
          setResults([...importResults]);
          continue;
        }

        if (result.success && result.project_id) {
          importResults.push({
            url,
            success: true,
            name: result.data?.name,
          });
          successProjectIds.push(result.project_id);
          setResults([...importResults]);

          invoke("post_import_tasks", {
            projectId: result.project_id,
            generateAiReport: generateAIReport,
            autoTranslate: autoTranslate,
            autoTranslateReadme: autoTranslate,
            defaultLanguage: settingsLang,
          }).catch((e) => console.error("后台任务失败:", e));
        } else {
          importResults.push({
            url,
            success: false,
            error: result.error || t("import.importFailedShort"),
          });
          setResults([...importResults]);
        }
      } catch (e) {
        importResults.push({
          url,
          success: false,
          error: String(e),
        });
        setResults([...importResults]);
      }

      setCurrentStep("");
    }

    setImporting(false);

    if (successProjectIds.length > 0) {
      onSuccess();
      onClose();
    }
  };

  const successCount = results.filter((r) => r.success).length;
  const failCount = results.filter((r) => !r.success && !r.duplicate).length;
  const duplicateCount = results.filter((r) => r.duplicate).length;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-2xl max-h-[90vh] overflow-hidden flex flex-col">
        <div className="p-6 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <h2 className="text-xl font-semibold">{t("import.title")}</h2>
            <button
              onClick={() => setShowManualAdd(true)}
              className="text-sm text-blue-600 dark:text-blue-400 hover:underline"
            >
              <i className="fa-solid fa-pen mr-1"></i>{t("import.manualAdd")}
            </button>
          </div>

          <div className="flex gap-4 mt-4">
            <button
              onClick={() => setMode("single")}
              disabled={importing}
              className={`px-4 py-2 rounded-lg ${
                mode === "single"
                  ? "bg-indigo-100 text-indigo-700 dark:bg-indigo-900 dark:text-indigo-300"
                  : "text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700"
              } disabled:cursor-not-allowed disabled:opacity-50`}
            >
              <i className="fa-solid fa-link mr-2"></i>
              {t("import.single")}
            </button>
            {false && (
            <button
              onClick={() => setMode("batch")}
              disabled={importing}
              className={`px-4 py-2 rounded-lg ${
                mode === "batch"
                  ? "bg-indigo-100 text-indigo-700 dark:bg-indigo-900 dark:text-indigo-300"
                  : "text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700"
              } disabled:cursor-not-allowed disabled:opacity-50`}
            >
              <i className="fa-solid fa-list mr-2"></i>
              {t("import.batch")}
            </button>
            )}
          </div>
        </div>

        <div className="flex-1 p-6 overflow-y-auto">
          {mode === "single" ? (
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2">
                  {t("import.url")}
                </label>
                <input
                  type="text"
                  value={singleUrl}
                  onChange={(e) => setSingleUrl(e.target.value)}
                  placeholder="https://github.com/facebook/react"
                  disabled={importing}
                  className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-indigo-500 disabled:bg-gray-100 disabled:cursor-not-allowed dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                />
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                  {t("import.autoPlatform")}
                </p>
              </div>

              <div>
                <label className="block text-sm font-medium mb-2">
                  {t("import.category")}
                </label>
                <select
                  value={categoryId ?? ""}
                  onChange={(e) => setCategoryId(e.target.value ? Number(e.target.value) : null)}
                  disabled={importing}
                  className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-indigo-500 disabled:bg-gray-100 disabled:cursor-not-allowed dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                >
                  <option value="">{t("import.autoSelectCategory")}</option>
                  {categories.map((cat) => (
                    <option key={cat.id} value={cat.id}>
                      {cat.name}
                    </option>
                  ))}
                </select>
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                  {t("import.autoCategoryTip")}
                </p>
              </div>

              <div className="space-y-2">
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={generateAIReport}
                    onChange={(e) => setGenerateAIReport(e.target.checked)}
                    disabled={importing}
                    className="rounded text-indigo-600 disabled:cursor-not-allowed"
                  />
                  <span className="text-sm">{t("import.generateAiReport")}</span>
                </label>

                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={autoTranslate}
                    onChange={(e) => setAutoTranslate(e.target.checked)}
                    disabled={importing}
                    className="rounded text-indigo-600 disabled:cursor-not-allowed"
                  />
                  <span className="text-sm">{t("import.autoTranslateReadme")}</span>
                </label>
              </div>
            </div>
          ) : (
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2">
                  {t("import.batchUrls")}
                </label>
                <textarea
                  value={urls}
                  onChange={(e) => setUrls(e.target.value)}
                  placeholder={t("import.batchPlaceholder")}
                  rows={10}
                  disabled={importing}
                  className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-indigo-500 font-mono text-sm disabled:bg-gray-100 disabled:cursor-not-allowed dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                />
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                  {t("import.batchTip", { count: urlList.length })}
                </p>
              </div>

              <div className="space-y-2">
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={generateAIReport}
                    onChange={(e) => setGenerateAIReport(e.target.checked)}
                    disabled={importing}
                    className="rounded text-indigo-600 disabled:cursor-not-allowed"
                  />
                  <span className="text-sm">{t("import.generateAiReportShort")}</span>
                </label>

                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={autoTranslate}
                    onChange={(e) => setAutoTranslate(e.target.checked)}
                    className="rounded text-indigo-600"
                  />
                  <span className="text-sm">{t("import.autoTranslateReadmeShort")}</span>
                </label>
              </div>
            </div>
          )}

          {error && (
            <div className="mt-4 p-4 bg-red-50 dark:bg-red-950 border border-red-200 dark:border-red-800 rounded-lg">
              <div className="flex items-start gap-3">
                <i className="fa-solid fa-circle-exclamation text-red-500 dark:text-red-400 mt-0.5"></i>
                <div>
                  <p className="font-medium text-red-700 dark:text-red-300">{t("import.importFailed")}</p>
                  <p className="text-sm text-red-600 dark:text-red-400 mt-1">{error}</p>
                </div>
              </div>
            </div>
          )}

          {results.length > 0 && (
            <div className="mt-4">
              <div className="flex items-center gap-4 mb-3">
                <span className="text-sm text-green-600 dark:text-green-400">
                  <i className="fa-solid fa-check-circle mr-1"></i>
                  {t("import.success", { count: successCount })}
                </span>
                {failCount > 0 && (
                  <span className="text-sm text-red-600 dark:text-red-400">
                    <i className="fa-solid fa-times-circle mr-1"></i>
                    {t("import.failed", { count: failCount })}
                  </span>
                )}
                {duplicateCount > 0 && (
                  <span className="text-sm text-yellow-600 dark:text-yellow-400">
                    <i className="fa-solid fa-exclamation-circle mr-1"></i>
                    {t("import.duplicateSkipped", { count: duplicateCount })}
                  </span>
                )}
              </div>

              <div className="border dark:border-gray-700 rounded-lg divide-y dark:divide-gray-700 max-h-60 overflow-y-auto">
                {results.map((result, index) => (
                  <div
                    key={index}
                    className="p-3 flex items-center gap-3"
                  >
                    {result.success ? (
                      <i className="fa-solid fa-check-circle text-green-500 dark:text-green-400"></i>
                    ) : result.duplicate ? (
                      <i className="fa-solid fa-exclamation-circle text-yellow-500 dark:text-yellow-400"></i>
                    ) : (
                      <i className="fa-solid fa-times-circle text-red-500 dark:text-red-400"></i>
                    )}
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium truncate">
                        {result.name || result.url}
                      </p>
                      {result.error && (
                        <p className={`text-xs truncate ${result.duplicate ? "text-yellow-600 dark:text-yellow-400" : "text-red-600 dark:text-red-400"}`}>
                          {result.error}
                        </p>
                      )}
                    </div>
                    {result.duplicate && (
                      <button
                        onClick={() => {
                          onSuccess();
                          onClose();
                          setTimeout(() => {
                            const event = new CustomEvent("openProjectDetail", { detail: { projectId: result.duplicate!.projectId } });
                            window.dispatchEvent(event);
                          }, 300);
                        }}
                        className="text-xs px-2 py-1 bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300 rounded hover:bg-blue-200 dark:hover:bg-blue-800 whitespace-nowrap"
                      >
                        {t("import.viewProject")}
                      </button>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-between">
          <div className="text-sm text-gray-500 dark:text-gray-400">
            {importing ? (
              <span className="flex items-center gap-2">
                <i className="fa-solid fa-spinner fa-spin text-blue-500 dark:text-blue-400"></i>
                {currentStep || t("import.importing")} ({currentIndex}/{urlList.length})
              </span>
            ) : results.length > 0 ? (
              <span>
                {duplicateCount > 0
                  ? t("import.withDuplicate", { success: successCount, fail: failCount, duplicate: duplicateCount })
                  : t("import.successFail", { success: successCount, fail: failCount })}
              </span>
            ) : null}
          </div>
          <div className="flex gap-3">
            <button
              onClick={onClose}
              disabled={importing}
              className="px-4 py-2 border dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50"
            >
              {t("import.cancel")}
            </button>
            <button
              onClick={handleImport}
              disabled={urlList.length === 0 || importing}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 min-w-[120px]"
            >
              {importing ? (
                <span className="flex items-center justify-center gap-2">
                  <i className="fa-solid fa-spinner fa-spin"></i>
                  <span>
                    {results.length > 0 && results[results.length - 1]?.name
                      ? t("import.importingName", { name: results[results.length - 1].name?.slice(0, 10) })
                      : t("import.importingProgress", { current: currentIndex, total: urlList.length })}
                    ({currentIndex}/{urlList.length})
                  </span>
                </span>
              ) : (
                t("import.importCount", { count: urlList.length })
              )}
            </button>
          </div>
        </div>
      </div>

      <ManualAddDialog
        open={showManualAdd}
        onClose={() => setShowManualAdd(false)}
        onSuccess={() => {
          setShowManualAdd(false);
          onSuccess();
        }}
      />
    </div>
  );
}
