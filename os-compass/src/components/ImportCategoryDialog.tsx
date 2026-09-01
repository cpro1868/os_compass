import { useState } from "react";
import { useTranslation } from "react-i18next";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { Category } from "../types";
import { categoryApi } from "../api/category";
import type { ParsedCategories, ImportSummary } from "../api/category";

interface ImportCategoryDialogProps {
  categories: Category[];
  onClose: () => void;
  onImported: () => void;
}

type Step = "select" | "settings" | "preview" | "executing" | "done";

const FILE_FILTERS = [
  { name: "分类导入文件", extensions: ["txt", "json"] },
];

export function ImportCategoryDialog({ categories, onClose, onImported }: ImportCategoryDialogProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState<Step>("select");
  const [filePath, setFilePath] = useState<string | null>(null);
  const [fileName, setFileName] = useState("");
  const [parentId, setParentId] = useState<number | null>(null);
  const [parsed, setParsed] = useState<ParsedCategories | null>(null);
  const [summary, setSummary] = useState<ImportSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showErrors, setShowErrors] = useState(false);
  const [loading, setLoading] = useState(false);

  const handleSelectFile = async () => {
    try {
      const selected = await openDialog({
        multiple: false,
        filters: FILE_FILTERS,
        title: t("category.selectFile"),
      });
      if (typeof selected === "string") {
        setFilePath(selected);
        setFileName(selected.split(/[\\/]/).pop() || selected);
        setError(null);
        setStep("settings");
      }
    } catch (e) {
      setError(String(e));
    }
  };

  const handlePreview = async () => {
    if (!filePath) return;
    setLoading(true);
    setError(null);
    try {
      const result = await categoryApi.parseFile(filePath, parentId);
      setParsed(result);
      setStep("preview");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleImport = async () => {
    if (!filePath) return;
    setLoading(true);
    setError(null);
    setStep("executing");
    try {
      const result = await categoryApi.importFile(filePath, parentId);
      setSummary(result);
      setStep("done");
    } catch (e) {
      setError(String(e));
      setStep("settings");
    } finally {
      setLoading(false);
    }
  };

  const handleDone = () => {
    onImported();
    onClose();
  };

  const total = parsed?.total ?? 0;
  const newCount = parsed?.items.filter((i) => i.status === "new").length ?? 0;
  const skipCount = parsed?.items.filter((i) => i.status === "skip").length ?? 0;
  const errCount = parsed?.items.filter((i) => i.status === "error").length ?? 0;
  const canImport = newCount > 0 || skipCount > 0;

  const renderParentOptions = (cats: Category[], level = 0): React.ReactNode[] => {
    const options: React.ReactNode[] = [];
    cats
      .filter((c) => !c.is_system)
      .sort((a, b) => (a.sort_order || 0) - (b.sort_order || 0))
      .forEach((cat) => {
        const prefix = "\u00A0\u00A0\u00A0\u00A0".repeat(level);
        options.push(
          <option key={cat.id} value={cat.id}>
            {prefix}{cat.name}
          </option>
        );
        options.push(
          ...renderParentOptions(categories.filter((c) => c.parent_id === cat.id), level + 1)
        );
      });
    return options;
  };

  const statusBadge = (status: string) => {
    switch (status) {
      case "new":
        return (
          <span className="px-2 py-0.5 rounded text-xs bg-emerald-100 text-emerald-700 dark:bg-emerald-900 dark:text-emerald-300">
            {t("category.statusNew")}
          </span>
        );
      case "skip":
        return (
          <span className="px-2 py-0.5 rounded text-xs bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-300">
            {t("category.statusSkipped")}
          </span>
        );
      default:
        return (
          <span className="px-2 py-0.5 rounded text-xs bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-300">
            {t("category.statusError")}
          </span>
        );
    }
  };

  return (
    <div className="fixed inset-0 z-[150] bg-black/50 flex items-center justify-center p-6">
      <div className="bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-2xl max-h-[85vh] flex flex-col">
        {/* 头部 */}
        <header className="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
          <h2 className="text-lg font-bold dark:text-gray-100">
            <i className="fa-solid fa-file-import mr-2 text-blue-500"></i>
            {t("category.importDialogTitle")}
          </h2>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
          >
            <i className="fa-solid fa-xmark"></i>
          </button>
        </header>

        {/* 步骤指示 */}
        <div className="px-6 pt-4">
          <div className="flex items-center gap-2 text-xs">
            {["select", "settings", "preview"].map((s, i) => (
              <div key={s} className="flex items-center gap-2">
                <div
                  className={`w-6 h-6 rounded-full flex items-center justify-center ${
                    step === s || (s === "settings" && (step === "preview" || step === "executing" || step === "done"))
                      ? "bg-blue-500 text-white"
                      : "bg-gray-200 dark:bg-gray-700 text-gray-500"
                  }`}
                >
                  {i + 1}
                </div>
                <span className={step === s ? "text-blue-600 dark:text-blue-400" : "text-gray-500"}>
                  {s === "select" ? t("category.selectFile") : s === "settings" ? t("category.importOptions") : t("category.preview")}
                </span>
                {i < 2 && <div className="w-6 h-px bg-gray-300 dark:bg-gray-600"></div>}
              </div>
            ))}
          </div>
        </div>

        {/* 内容区 */}
        <div className="flex-1 overflow-auto px-6 py-4">
          {error && (
            <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/30 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-600 dark:text-red-300">
              <i className="fa-solid fa-circle-exclamation mr-1"></i>{error}
            </div>
          )}

          {/* 步骤1: 选文件 */}
          {step === "select" && (
            <div className="text-center py-8">
              <div className="w-16 h-16 bg-blue-100 dark:bg-blue-900 rounded-2xl flex items-center justify-center mx-auto mb-4">
                <i className="fa-solid fa-file-import text-blue-500 text-2xl"></i>
              </div>
              <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">{t("category.fileFormatHint")}</p>
              <button
                onClick={handleSelectFile}
                className="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition"
              >
                <i className="fa-solid fa-folder-open mr-2"></i>{t("category.selectFile")}
              </button>
            </div>
          )}

          {/* 步骤2: 导入设置 */}
          {step === "settings" && (
            <div className="space-y-5">
              <div className="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg border border-gray-200 dark:border-gray-600 flex items-center gap-3">
                <i className="fa-solid fa-file-lines text-blue-500"></i>
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium dark:text-gray-200 truncate">{fileName}</div>
                  <div className="text-xs text-gray-500 dark:text-gray-400 truncate">{filePath}</div>
                </div>
                <button
                  onClick={handleSelectFile}
                  className="text-xs text-blue-600 dark:text-blue-400 hover:underline"
                >
                  {t("category.selectFile")}
                </button>
              </div>

              <div>
                <label className="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">
                  {t("category.parentCategory")}
                </label>
                <select
                  value={parentId ?? ""}
                  onChange={(e) => setParentId(e.target.value ? Number(e.target.value) : null)}
                  className="w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                >
                  <option value="">{t("category.parentCategoryTop")}</option>
                  {renderParentOptions(categories.filter((c) => c.parent_id === null))}
                </select>
              </div>

              <div className="p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg text-sm text-amber-700 dark:text-amber-300">
                <i className="fa-solid fa-info-circle mr-1"></i>{t("category.skipHint")}
              </div>

              <div className="flex justify-end gap-3 pt-2">
                <button
                  onClick={onClose}
                  className="px-4 py-2 border rounded-lg hover:bg-gray-100 dark:border-gray-600 dark:hover:bg-gray-700"
                >
                  {t("category.form.cancel")}
                </button>
                <button
                  onClick={handlePreview}
                  disabled={loading}
                  className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
                >
                  {loading ? <i className="fa-solid fa-spinner fa-spin"></i> : <>{t("category.preview")}</>}
                </button>
              </div>
            </div>
          )}

          {/* 步骤3: 预览 */}
          {step === "preview" && parsed && (
            <div>
              <div className="mb-4 p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg text-sm dark:text-gray-200">
                {t("category.previewStats", {
                  total,
                  created: newCount,
                  skipped: skipCount,
                  errors: errCount,
                })}
              </div>

              <div className="max-h-64 overflow-auto border border-gray-200 dark:border-gray-600 rounded-lg">
                <table className="w-full text-sm">
                  <thead className="bg-gray-100 dark:bg-gray-700 sticky top-0">
                    <tr>
                      <th className="px-3 py-2 text-left dark:text-gray-200">{t("category.form.name")}</th>
                      <th className="px-3 py-2 text-right dark:text-gray-200">{t("category.preview")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {parsed.items.map((item, idx) => (
                      <tr key={idx} className="border-t border-gray-100 dark:border-gray-700">
                        <td className="px-3 py-1.5 dark:text-gray-200">{item.full_path}</td>
                        <td className="px-3 py-1.5 text-right">{statusBadge(item.status)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              {errCount > 0 && (
                <button
                  onClick={() => setShowErrors(!showErrors)}
                  className="mt-2 text-xs text-red-600 dark:text-red-400 hover:underline"
                >
                  <i className="fa-solid fa-chevron-down mr-1"></i>
                  {t("category.showErrors")} ({errCount})
                </button>
              )}
              {showErrors && (
                <div className="mt-2 p-3 bg-red-50 dark:bg-red-900/20 rounded-lg text-xs text-red-700 dark:text-red-300 space-y-1">
                  {parsed.items
                    .filter((i) => i.status === "error")
                    .map((i, idx) => (
                      <div key={idx}>
                        <span className="font-medium">#{i.source_line}</span> {i.full_path}: {i.reason}
                      </div>
                    ))}
                </div>
              )}

              <div className="flex justify-between gap-3 pt-4">
                <button
                  onClick={() => setStep("settings")}
                  className="px-4 py-2 border rounded-lg hover:bg-gray-100 dark:border-gray-600 dark:hover:bg-gray-700"
                >
                  {t("category.back")}
                </button>
                <button
                  onClick={handleImport}
                  disabled={!canImport}
                  className="px-4 py-2 bg-emerald-600 text-white rounded-lg hover:bg-emerald-700 disabled:opacity-50"
                >
                  <i className="fa-solid fa-check mr-1"></i>{t("category.confirmImport")}
                </button>
              </div>
            </div>
          )}

          {/* 步骤4: 执行中 */}
          {step === "executing" && (
            <div className="text-center py-10">
              <i className="fa-solid fa-spinner fa-spin text-3xl text-blue-500 mb-4"></i>
              <p className="text-sm text-gray-500 dark:text-gray-400">{t("category.importing")}</p>
            </div>
          )}

          {/* 步骤5: 完成 */}
          {step === "done" && summary && (
            <div className="text-center py-6">
              <div className="w-16 h-16 bg-emerald-100 dark:bg-emerald-900 rounded-full flex items-center justify-center mx-auto mb-4">
                <i className="fa-solid fa-check text-emerald-500 text-2xl"></i>
              </div>
              <p className="font-medium mb-2 dark:text-gray-100">
                {t("category.importDone", {
                  created: summary.created,
                  skipped: summary.skipped,
                  errors: summary.errors.length,
                })}
              </p>
              {summary.errors.length > 0 && (
                <div className="mt-3 max-h-40 overflow-auto p-3 bg-red-50 dark:bg-red-900/20 rounded-lg text-xs text-red-700 dark:text-red-300 text-left space-y-1">
                  {summary.errors.map((e, idx) => (
                    <div key={idx}>
                      <span className="font-medium">#{e.source_line}</span> {e.full_path}: {e.reason}
                    </div>
                  ))}
                </div>
              )}
              <button
                onClick={handleDone}
                className="mt-5 px-6 py-2.5 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition"
              >
                {t("common.close")}
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
