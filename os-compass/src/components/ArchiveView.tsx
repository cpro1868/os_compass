import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Project } from "../types";
import { EmptyState } from "./EmptyState";

interface ArchiveViewProps {
  onBack: () => void;
  projects: Project[];
  onRestore: (id: number) => void;
  onPermanentDelete: (id: number) => void;
  onBatchRestore: (ids: number[]) => void;
  onBatchPermanentDelete: (ids: number[]) => void;
}

export function ArchiveView({
  onBack,
  projects,
  onRestore,
  onPermanentDelete,
  onBatchRestore,
  onBatchPermanentDelete,
}: ArchiveViewProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [showDeleteModal, setShowDeleteModal] = useState(false);

  const filteredProjects = projects.filter((p) => {
    if (!searchQuery) return true;
    const query = searchQuery.toLowerCase();
    return (
      p.name.toLowerCase().includes(query) ||
      p.url?.toLowerCase().includes(query) ||
      p.description?.toLowerCase().includes(query)
    );
  });

  const toggleSelect = (id: number) => {
    const newSet = new Set(selectedIds);
    if (newSet.has(id)) {
      newSet.delete(id);
    } else {
      newSet.add(id);
    }
    setSelectedIds(newSet);
  };

  const toggleSelectAll = () => {
    if (selectedIds.size === filteredProjects.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(filteredProjects.map((p) => p.id)));
    }
  };

  const handleBatchDelete = () => {
    onBatchPermanentDelete(Array.from(selectedIds));
    setSelectedIds(new Set());
    setShowDeleteModal(false);
  };

  return (
    <div className="flex flex-col h-full">
      {/* 批量操作栏 */}
      {selectedIds.size > 0 && (
        <div className="bg-amber-600 text-white px-6 py-3 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <span>{t("archive.selected", { count: selectedIds.size })}</span>
            <button
              onClick={toggleSelectAll}
              className="text-sm underline hover:text-amber-100 dark:hover:text-amber-200"
            >
              {selectedIds.size === filteredProjects.length ? t("archive.cancelSelect") : t("archive.selectAll")}
            </button>
            <button
              onClick={() => setSelectedIds(new Set())}
              className="text-sm underline hover:text-amber-100 dark:hover:text-amber-200"
            >
              {t("archive.deselect")}
            </button>
          </div>
          <div className="flex items-center gap-3">
            <button
              onClick={() => {
                onBatchRestore(Array.from(selectedIds));
                setSelectedIds(new Set());
              }}
              className="px-3 py-1.5 bg-white/20 hover:bg-white/30 rounded-lg text-sm flex items-center gap-2"
            >
              <i className="fa-solid fa-rotate-left mr-2"></i>{t("archive.batchRestore")}
            </button>
            <button
              onClick={() => setShowDeleteModal(true)}
              className="px-3 py-1.5 bg-red-600 hover:bg-red-700 rounded-lg text-sm flex items-center gap-2"
            >
              <i className="fa-solid fa-trash mr-2"></i>{t("archive.batchDelete")}
            </button>
          </div>
        </div>
      )}

      {/* 顶部栏 */}
      <header className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-4">
            <button
              onClick={onBack}
              className="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
            >
              <i className="fa-solid fa-arrow-left mr-2"></i>{t("common.close")}
            </button>
            <i className="fa-solid fa-box-archive text-amber-600 dark:text-amber-400"></i>
            <h1 className="text-xl font-bold dark:text-gray-100">{t("archive.title")}</h1>
            <span className="text-sm text-gray-500 dark:text-gray-400">
              {t("kanban.totalProjects", { count: filteredProjects.length })}
            </span>
          </div>
        </div>

        {/* 搜索框 */}
        <div className="relative">
          <i className="fa-solid fa-search absolute left-3 top-1/2 -translate-y-1/2 text-gray-400 dark:text-gray-500"></i>
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t("archive.search")}
            className="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
          />
        </div>
      </header>

      {/* 内容区 */}
      <div className="flex-1 overflow-auto p-6">
        {filteredProjects.length === 0 ? (
          projects.length === 0 ? (
            <EmptyState
              icon="fa-box-archive"
              title={t("archive.empty")}
              description={t("archive.empty")}
              actionLabel={t("archive.returnToList")}
              onAction={onBack}
            />
          ) : (
            <EmptyState
              icon="fa-magnifying-glass"
              title={t("archive.notFound")}
              description={t("archive.adjustSearch")}
            />
          )
        ) : (
          <div className="space-y-4">
            {filteredProjects.map((p) => (
              <div
                key={p.id}
                className={`bg-white dark:bg-gray-800 rounded-xl border p-4 hover:shadow-md transition ${
                  selectedIds.has(p.id) ? "border-amber-300 bg-amber-50 dark:border-amber-600 dark:bg-amber-950" : "border-gray-200 dark:border-gray-700"
                }`}
              >
                <div className="flex items-start gap-4">
                  <input
                    type="checkbox"
                    checked={selectedIds.has(p.id)}
                    onChange={() => toggleSelect(p.id)}
                    className="mt-1 rounded"
                  />

                  <div className="w-12 h-12 bg-gray-100 dark:bg-gray-700 rounded-xl flex items-center justify-center text-gray-400 dark:text-gray-500">
                    <i className="fa-solid fa-box-archive text-xl"></i>
                  </div>

                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-3">
                      <h3 className="font-medium text-gray-700 dark:text-gray-300">{p.name}</h3>
                      <span className="px-2 py-0.5 text-xs bg-amber-100 text-amber-700 dark:bg-amber-900 dark:text-amber-300 rounded">
                        {t("archive.archived")}
                      </span>
                    </div>
                    <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">{p.url}</p>
                    <div className="flex items-center gap-4 mt-2 text-xs text-gray-400 dark:text-gray-500">
                      <span>
                        <i className="fa-solid fa-folder mr-1"></i>
                        {p.category_id || "-"}
                      </span>
                      <span>
                        <i className="fa-solid fa-clock mr-1"></i>
                        {t("archive.archivedAt")} {p.archived_at ? new Date(p.archived_at).toLocaleDateString() : "-"}
                      </span>
                      <span>
                        <i className="fa-solid fa-calendar mr-1"></i>
                        {t("archive.originalUpdated")} {new Date(p.updated_at).toLocaleDateString()}
                      </span>
                    </div>
                  </div>

                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => onRestore(p.id)}
                      className="px-3 py-1.5 text-sm border border-green-600 text-green-600 dark:text-green-400 dark:border-green-500 rounded-lg hover:bg-green-50 dark:hover:bg-green-950 flex items-center gap-2"
                    >
                      <i className="fa-solid fa-rotate-left"></i>{t("actions.restore")}
                    </button>
                    <button
                      onClick={() => onPermanentDelete(p.id)}
                      className="px-3 py-1.5 text-sm border border-red-600 text-red-600 dark:text-red-400 dark:border-red-500 rounded-lg hover:bg-red-50 dark:hover:bg-red-950 flex items-center gap-2"
                    >
                      <i className="fa-solid fa-trash"></i>{t("archive.deletePermanent")}
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* 永久删除确认弹窗 */}
      {showDeleteModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white dark:bg-gray-800 rounded-xl p-6 w-full max-w-md shadow-xl">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-12 h-12 bg-red-100 dark:bg-red-900 rounded-full flex items-center justify-center">
                <i className="fa-solid fa-triangle-exclamation text-red-600 dark:text-red-400 text-xl"></i>
              </div>
              <div>
                <h3 className="font-semibold text-lg dark:text-gray-100">{t("archive.deleteConfirmTitle")}</h3>
                <p className="text-sm text-gray-500 dark:text-gray-400">{t("archive.deleteConfirmSubtitle")}</p>
              </div>
            </div>

            <p className="text-gray-600 dark:text-gray-300 mb-4">
              {t("archive.deleteConfirmMsg", { count: selectedIds.size })}
            </p>

            <p className="text-sm text-red-500 dark:text-red-400 mb-4">
              <i className="fa-solid fa-exclamation-circle mr-1"></i>
              {t("archive.deleteWarning")}
            </p>
            <ul className="text-sm text-gray-500 dark:text-gray-400 mb-6 list-disc ml-4">
              <li>{t("archive.deleteWarningItem1")}</li>
              <li>{t("archive.deleteWarningItem2")}</li>
              <li>{t("archive.deleteWarningItem3")}</li>
              <li>{t("archive.deleteWarningItem4")}</li>
            </ul>

            <div className="flex justify-end gap-3">
              <button
                onClick={() => setShowDeleteModal(false)}
                className="px-4 py-2 border rounded-lg hover:bg-gray-50 dark:border-gray-600 dark:hover:bg-gray-700"
              >
                {t("common.cancel")}
              </button>
              <button
                onClick={handleBatchDelete}
                className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700"
              >
                {t("archive.confirmDelete")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
