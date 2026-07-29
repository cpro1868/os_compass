import { useState, useMemo, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { Project, LifecycleStatus, Category } from "../types";
import { parseLanguages } from "../types";
import { getCategories } from "../api";
import { MoveCategoryDialog } from "./MoveCategoryDialog";
import { EmptyState } from "./EmptyState";
import { useToastStore } from "../stores/toastStore";

interface KanbanViewProps {
  projects: Project[];
  onProjectClick: (project: Project) => void;
  onStatusChange: (projectId: number, newStatus: LifecycleStatus) => void;
  selectedIds?: Set<number>;
  onSelectionChange?: (ids: Set<number>) => void;
  onBatchMoveCategory?: (ids: number[], categoryId: number | null) => void;
  onBatchArchive?: (ids: number[]) => void;
  onBatchExport?: (projects: Project[]) => void;
}

type SortField = "name" | "stars" | "updated_at" | "created_at";
type SortOrder = "asc" | "desc";

const STATUS_CONFIG: Record<LifecycleStatus, { emoji: string; title: string }> = {
  TO_EXPLORE: { emoji: "💡", title: "待探索" },
  DIVING: { emoji: "🔬", title: "深度研究中" },
  IN_USE: { emoji: "✅", title: "已落地/在用" },
  ABANDONED: { emoji: "🗑️", title: "弃用/避坑" },
};

export function KanbanView({ projects, onProjectClick, onStatusChange, selectedIds = new Set(), onSelectionChange, onBatchMoveCategory, onBatchArchive, onBatchExport }: KanbanViewProps) {
  const { t } = useTranslation();
  const { showToast } = useToastStore();
  const [dragOverColumn, setDragOverColumn] = useState<LifecycleStatus | null>(null);
  const [showFilter, setShowFilter] = useState(false);
  const [showSort, setShowSort] = useState(false);
  const [filterCategory, setFilterCategory] = useState<number | null>(null);
  const [filterLanguage, setFilterLanguage] = useState<string>("");
  const [sortField, setSortField] = useState<SortField>("created_at");
  const [sortOrder, setSortOrder] = useState<SortOrder>("desc");
  const [categories, setCategories] = useState<Category[]>([]);
  const [draggingProject, setDraggingProject] = useState<{ project: Project; offsetX: number; offsetY: number } | null>(null);
  const [dragPos, setDragPos] = useState<{ x: number; y: number } | null>(null);
  const [showMoveDialog, setShowMoveDialog] = useState(false);
  const [vectorizing, setVectorizing] = useState(false);
  const dragStateRef = useRef<{ projectId: number; fromStatus: LifecycleStatus } | null>(null);
  const columnsRef = useRef<Map<LifecycleStatus, HTMLDivElement>>(new Map());

  const hasSelection = selectedIds.size > 0;

  const handleBatchVectorize = async () => {
    if (selectedIds.size === 0) {
      showToast("请先选择要向量化的项目", "info");
      return;
    }
    if (!confirm(`确认向量化选中的 ${selectedIds.size} 个项目？`)) return;

    setVectorizing(true);
    try {
      const selectedProjects = projects.filter(p => selectedIds.has(p.id));
      let success = 0;
      for (const project of selectedProjects) {
        await invoke("generate_project_embeddings", { projectId: project.id });
        success++;
      }
      showToast(`成功向量化 ${success} 个项目`, "success");
    } catch (err) {
      console.error("批量向量化失败:", err);
      showToast("向量化失败：" + String(err), "error");
    } finally {
      setVectorizing(false);
    }
  };

  const handleVectorizeAll = async () => {
    if (!confirm(`确认向量化当前仓库的所有 ${projects.length} 个项目？`)) return;

    setVectorizing(true);
    try {
      const count = await invoke<number>("rebuild_embeddings", { projectId: null });
      showToast(`成功向量化 ${count} 个项目`, "success");
    } catch (err) {
      console.error("全量向量化失败:", err);
      showToast("向量化失败：" + String(err), "error");
    } finally {
      setVectorizing(false);
    }
  };

  const toggleSelect = (id: number, e: React.MouseEvent) => {
    e.stopPropagation();
    const newSet = new Set(selectedIds);
    if (newSet.has(id)) {
      newSet.delete(id);
    } else {
      newSet.add(id);
    }
    onSelectionChange?.(newSet);
  };

  const toggleSelectAll = () => {
    if (selectedIds.size === filteredProjects.length) {
      onSelectionChange?.(new Set());
    } else {
      onSelectionChange?.(new Set(filteredProjects.map((p) => p.id)));
    }
  };

  const handleBatchArchive = async () => {
    if (!confirm(`确认归档 ${selectedIds.size} 个项目？`)) return;
    try {
      await onBatchArchive?.(Array.from(selectedIds));
      onSelectionChange?.(new Set());
    } catch (err) {
      console.error("批量归档失败:", err);
    }
  };

  const handleBatchExport = () => {
    if (onBatchExport) {
      const selected = filteredProjects.filter((p) => selectedIds.has(p.id));
      onBatchExport(selected);
    }
  };

  useEffect(() => {
    getCategories().then(setCategories).catch(() => {});
  }, []);

  useEffect(() => {
    if (!draggingProject) return;
    const handleMove = (e: PointerEvent) => {
      setDragPos({ x: e.clientX, y: e.clientY });
      const target = document.elementFromPoint(e.clientX, e.clientY);
      const column = target?.closest("[data-column]") as HTMLDivElement | null;
      if (column) {
        const status = column.dataset.column as LifecycleStatus;
        setDragOverColumn(status);
      } else {
        setDragOverColumn(null);
      }
    };
    const handleUp = (e: PointerEvent) => {
      const target = document.elementFromPoint(e.clientX, e.clientY);
      const column = target?.closest("[data-column]") as HTMLDivElement | null;
      if (column) {
        const status = column.dataset.column as LifecycleStatus;
        if (draggingProject.project.lifecycle_status !== status) {
          onStatusChange(draggingProject.project.id, status);
        }
      }
      setDraggingProject(null);
      setDragPos(null);
      setDragOverColumn(null);
      dragStateRef.current = null;
    };
    window.addEventListener("pointermove", handleMove);
    window.addEventListener("pointerup", handleUp);
    return () => {
      window.removeEventListener("pointermove", handleMove);
      window.removeEventListener("pointerup", handleUp);
    };
  }, [draggingProject, onStatusChange]);

  const STATUSES: LifecycleStatus[] = ["TO_EXPLORE", "DIVING", "IN_USE", "ABANDONED"];

  const languages = useMemo(() => {
    const langSet = new Set<string>();
    projects.forEach((p) => {
      parseLanguages(p.languages).forEach((l) => langSet.add(l));
    });
    return Array.from(langSet).sort();
  }, [projects]);

  const filteredProjects = useMemo(() => {
    let result = [...projects];
    if (filterCategory) {
      result = result.filter((p) => p.category_id === filterCategory);
    }
    if (filterLanguage) {
      result = result.filter((p) => parseLanguages(p.languages).includes(filterLanguage));
    }
    result.sort((a, b) => {
      let comparison = 0;
      switch (sortField) {
        case "name": comparison = a.name.localeCompare(b.name); break;
        case "stars": comparison = (a.stars || 0) - (b.stars || 0); break;
        case "updated_at": comparison = new Date(a.updated_at).getTime() - new Date(b.updated_at).getTime(); break;
        case "created_at": comparison = new Date(a.created_at).getTime() - new Date(a.created_at).getTime(); break;
      }
      return sortOrder === "asc" ? comparison : -comparison;
    });
    return result;
  }, [projects, filterCategory, filterLanguage, sortField, sortOrder]);

  const groupedProjects = useMemo(() => {
    const groups: Record<LifecycleStatus, Project[]> = {
      TO_EXPLORE: [],
      DIVING: [],
      IN_USE: [],
      ABANDONED: [],
    };
    filteredProjects.forEach((p) => {
      const status = p.lifecycle_status as LifecycleStatus;
      if (groups[status]) {
        groups[status].push(p);
      } else {
        groups.TO_EXPLORE.push(p);
      }
    });
    return groups;
  }, [filteredProjects]);

  const handleCardPointerDown = (e: React.PointerEvent, project: Project) => {
    if (e.button !== 0) return;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    setDraggingProject({ project, offsetX: e.clientX - rect.left, offsetY: e.clientY - rect.top });
    setDragPos({ x: e.clientX, y: e.clientY });
    dragStateRef.current = { projectId: project.id, fromStatus: project.lifecycle_status as LifecycleStatus };
    e.preventDefault();
  };

  const handleColumnRef = (status: LifecycleStatus, el: HTMLDivElement | null) => {
    if (el) {
      columnsRef.current.set(status, el);
    } else {
      columnsRef.current.delete(status);
    }
  };

  const clearFilters = () => {
    setFilterCategory(null);
    setFilterLanguage("");
  };

  const hasActiveFilters = filterCategory !== null || filterLanguage !== "";

  return (
    <div className="flex flex-col h-full">
      <header className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4 flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold text-gray-900 dark:text-gray-100">{t("kanban.title")}</h1>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            {t("kanban.totalProjects", { count: filteredProjects.length })}
            {hasActiveFilters && <span className="text-blue-600 dark:text-blue-400">{t("kanban.filtered")}</span>}
          </p>
        </div>
        <div className="flex gap-2 relative">
          <div className="relative">
            <button
              onClick={() => { setShowFilter(!showFilter); setShowSort(false); }}
              className={`px-3 py-1.5 text-sm border rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 flex items-center gap-1 dark:text-gray-300 ${
                hasActiveFilters ? "bg-blue-50 border-blue-300 text-blue-700 dark:bg-blue-900 dark:border-blue-700 dark:text-blue-300" : "dark:border-gray-600"
              }`}
            >
              <i className="fa-solid fa-filter"></i>
              {t("kanban.filter")}
              {hasActiveFilters && (
                <span className="ml-1 w-5 h-5 bg-blue-600 text-white text-xs rounded-full flex items-center justify-center">
                  {(filterCategory ? 1 : 0) + (filterLanguage ? 1 : 0)}
                </span>
              )}
            </button>
            {showFilter && (
              <div className="absolute right-0 top-full mt-2 w-64 bg-white border rounded-lg shadow-lg z-10 p-4">
                <div className="mb-3">
                  <label className="block text-sm font-medium mb-1">{t("kanban.category")}</label>
                  <select
                    value={filterCategory ?? ""}
                    onChange={(e) => setFilterCategory(e.target.value ? Number(e.target.value) : null)}
                    className="w-full px-3 py-2 border rounded-lg text-sm dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                  >
                    <option value="">{t("kanban.all")}</option>
                    {categories.map((cat) => (
                      <option key={cat.id} value={cat.id}>{cat.name}</option>
                    ))}
                  </select>
                </div>
                <div className="mb-3">
                  <label className="block text-sm font-medium mb-1">{t("kanban.language")}</label>
                  <select
                    value={filterLanguage}
                    onChange={(e) => setFilterLanguage(e.target.value)}
                    className="w-full px-3 py-2 border rounded-lg text-sm dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                  >
                    <option value="">{t("kanban.all")}</option>
                    {languages.map((lang) => (
                      <option key={lang} value={lang}>{lang}</option>
                    ))}
                  </select>
                </div>
                {hasActiveFilters && (
                  <button onClick={clearFilters} className="text-sm text-blue-600 hover:underline">
                    {t("kanban.clearFilter")}
                  </button>
                )}
              </div>
            )}
          </div>
          <button
            onClick={handleVectorizeAll}
            disabled={vectorizing}
            className="px-3 py-1.5 text-sm border border-purple-200 bg-purple-50 hover:bg-purple-100 dark:bg-purple-900/30 dark:border-purple-700 rounded-lg flex items-center gap-1"
          >
            {vectorizing ? (
              <i className="fa-solid fa-spinner fa-spin text-purple-600"></i>
            ) : (
              <i className="fa-solid fa-brain text-purple-600"></i>
            )}
            <span className="text-purple-700 dark:text-purple-400">{vectorizing ? "向量化中..." : "向量化全部"}</span>
          </button>
          <div className="relative">
            <button
              onClick={() => { setShowSort(!showSort); setShowFilter(false); }}
              className="px-3 py-1.5 text-sm border rounded-lg hover:bg-gray-50 flex items-center gap-1"
            >
              <i className="fa-solid fa-sort"></i>
              {t("kanban.sort")}
            </button>
            {showSort && (
              <div className="absolute right-0 top-full mt-2 w-48 bg-white border rounded-lg shadow-lg z-10">
                <div className="py-1">
                  <button
                    onClick={() => { setSortField("created_at"); setShowSort(false); }}
                    className={`w-full px-4 py-2 text-left text-sm hover:bg-gray-50 ${
                      sortField === "created_at" ? "bg-blue-50 text-blue-700" : ""
                    }`}
                  >
                    {t("kanban.createTime")} {sortField === "created_at" && (sortOrder === "desc" ? "↓" : "↑")}
                  </button>
                  <button
                    onClick={() => { setSortField("updated_at"); setShowSort(false); }}
                    className={`w-full px-4 py-2 text-left text-sm hover:bg-gray-50 ${
                      sortField === "updated_at" ? "bg-blue-50 text-blue-700" : ""
                    }`}
                  >
                    {t("kanban.updateTime")} {sortField === "updated_at" && (sortOrder === "desc" ? "↓" : "↑")}
                  </button>
                  <button
                    onClick={() => { setSortField("name"); setShowSort(false); }}
                    className={`w-full px-4 py-2 text-left text-sm hover:bg-gray-50 ${
                      sortField === "name" ? "bg-blue-50 text-blue-700" : ""
                    }`}
                  >
                    {t("kanban.nameAZ")} {sortField === "name" && (sortOrder === "asc" ? "A→Z" : "Z→A")}
                  </button>
                  <button
                    onClick={() => { setSortField("stars"); setShowSort(false); }}
                    className={`w-full px-4 py-2 text-left text-sm hover:bg-gray-50 ${
                      sortField === "stars" ? "bg-blue-50 text-blue-700" : ""
                    }`}
                  >
                    {t("kanban.stars")} {sortField === "stars" && (sortOrder === "desc" ? "↓" : "↑")}
                  </button>
                </div>
                <div className="border-t px-4 py-2 flex gap-2">
                  <button
                    onClick={() => setSortOrder("desc")}
                    className={`flex-1 py-1 text-xs rounded ${
                      sortOrder === "desc" ? "bg-gray-200" : "hover:bg-gray-100"
                    }`}
                  >
                    {t("kanban.desc")} ↓
                  </button>
                  <button
                    onClick={() => setSortOrder("asc")}
                    className={`flex-1 py-1 text-xs rounded ${
                      sortOrder === "asc" ? "bg-gray-200" : "hover:bg-gray-100"
                    }`}
                  >
                    {t("kanban.asc")} ↑
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      </header>

      {hasSelection && (
        <div className="bg-blue-600 text-white px-6 py-3 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <span>已选中 {selectedIds.size} 项</span>
            <button
              onClick={toggleSelectAll}
              className="text-sm underline hover:text-blue-100"
            >
              {selectedIds.size === filteredProjects.length ? "取消全选" : "全选"}
            </button>
            <button
              onClick={() => onSelectionChange?.(new Set())}
              className="text-sm underline hover:text-blue-100"
            >
              取消选择
            </button>
          </div>
          <div className="flex items-center gap-3">
            <button
              onClick={() => setShowMoveDialog(true)}
              className="px-3 py-1.5 bg-white/20 hover:bg-white/30 rounded-lg text-sm flex items-center gap-2"
            >
              <i className="fa-solid fa-folder"></i>移动到分类
            </button>
            <button
              onClick={handleBatchArchive}
              className="px-3 py-1.5 bg-white/20 hover:bg-white/30 rounded-lg text-sm flex items-center gap-2"
            >
              <i className="fa-solid fa-box"></i>归档
            </button>
            <button
              onClick={handleBatchVectorize}
              disabled={vectorizing}
              className="px-3 py-1.5 bg-purple-500/80 hover:bg-purple-500 disabled:opacity-50 rounded-lg text-sm flex items-center gap-2"
            >
              {vectorizing ? (
                <i className="fa-solid fa-spinner fa-spin"></i>
              ) : (
                <i className="fa-solid fa-brain"></i>
              )}
              向量化{hasSelection ? `选中(${selectedIds.size})` : ""}
            </button>
            <button
              onClick={handleBatchExport}
              className="px-3 py-1.5 bg-white/20 hover:bg-white/30 rounded-lg text-sm flex items-center gap-2"
            >
              <i className="fa-solid fa-download"></i>导出
            </button>
          </div>
        </div>
      )}

      <div className="flex-1 overflow-auto p-6">
        {filteredProjects.length === 0 && !hasActiveFilters ? (
          <EmptyState
            icon="fa-inbox"
            title={t("kanban.emptyTitle")}
            description={t("kanban.emptyDesc")}
            actionLabel={t("kanban.emptyAction")}
            onAction={() => window.dispatchEvent(new CustomEvent("openImport"))}
          />
        ) : filteredProjects.length === 0 ? (
          <EmptyState
            icon="fa-magnifying-glass"
            title={t("kanban.noMatchTitle")}
            description={t("kanban.noMatchDesc")}
            actionLabel={t("kanban.noMatchAction")}
            onAction={clearFilters}
          />
        ) : (
          <>
        <div className="bg-blue-50 dark:bg-blue-950 border border-blue-200 dark:border-blue-800 rounded-lg px-4 py-2 mb-4 flex items-center gap-2 text-sm text-blue-700 dark:text-blue-400">
          <i className="fa-solid fa-hand-pointer"></i>
          {t("kanban.dropTip")}
        </div>

        <div className="grid grid-cols-4 gap-4">
          {STATUSES.map((status) => (
            <div
              key={status}
              data-column={status}
              ref={(el) => handleColumnRef(status, el)}
              className={`bg-gray-50 dark:bg-gray-800 rounded-xl p-4 flex flex-col min-h-[400px] ${
                dragOverColumn === status ? "ring-2 ring-indigo-400 bg-indigo-50 dark:bg-indigo-950" : ""
              }`}
            >
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-2">
                  <span className="text-lg">{STATUS_CONFIG[status].emoji}</span>
                  <h3 className="font-bold">{STATUS_CONFIG[status].title}</h3>
                </div>
                <span className="text-sm text-gray-500">
                  {groupedProjects[status]?.length || 0}
                </span>
              </div>

              <div className="flex-1 space-y-3 overflow-auto">
                {groupedProjects[status]?.map((project) => (
                  <div
                    key={project.id}
                    className={`block bg-white dark:bg-gray-700 rounded-lg border border-gray-200 dark:border-gray-600 hover:shadow-md hover:border-indigo-300 ${
                      status === "ABANDONED" ? "opacity-75" : ""} ${draggingProject?.project.id === project.id ? "opacity-30" : ""} ${
                      selectedIds.has(project.id) ? "ring-2 ring-blue-500 border-blue-500" : ""
                    }`}
                    onPointerDown={(e) => {
                      if (hasSelection) return;
                      handleCardPointerDown(e, project);
                    }}
                    style={{ touchAction: "none", cursor: hasSelection ? "default" : (draggingProject ? "grabbing" : "grab") }}
                  >
                    <div className="flex items-center gap-2 min-w-0 flex-1 p-4">
                      {hasSelection ? (
                        <input
                          type="checkbox"
                          checked={selectedIds.has(project.id)}
                          onChange={() => {}}
                          onClick={(e) => toggleSelect(project.id, e as unknown as React.MouseEvent)}
                          className="w-4 h-4 rounded flex-shrink-0 cursor-pointer"
                        />
                      ) : (
                        <div className="flex items-center justify-center flex-shrink-0 cursor-grab active:cursor-grabbing select-none">
                          <i className="fa-solid fa-grip-vertical text-gray-400"></i>
                        </div>
                      )}
                      <div
                        onClick={(e) => {
                          if (hasSelection) {
                            toggleSelect(project.id, e as unknown as React.MouseEvent);
                          } else {
                            e.stopPropagation();
                            onProjectClick(project);
                          }
                        }}
                        className="flex-1 text-left min-w-0 cursor-pointer"
                      >
                        <h4 className="font-medium text-sm truncate">{project.name}</h4>
                        <p className="text-xs text-gray-500">{project.source}</p>
                      </div>
                      {project.health_score !== null && project.health_score !== undefined && (
                        <div className={`flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold ${
                          project.health_score >= 80 ? "bg-green-100 text-green-700" :
                          project.health_score >= 60 ? "bg-blue-100 text-blue-700" :
                          project.health_score >= 40 ? "bg-yellow-100 text-yellow-700" :
                          "bg-red-100 text-red-700"
                        }`}>
                          {Math.round(project.health_score)}
                        </div>
                      )}
                    </div>
                    <div className="px-4 pb-4">
                      <p className="text-xs text-gray-600 line-clamp-2">
                        {project.description || "-"}
                      </p>
                      <div className="flex items-center gap-3 mt-2 text-xs text-gray-500">
                        {(() => {
                          const langs = parseLanguages(project.languages);
                          if (langs.length === 0) return null;
                          return (
                            <span className="flex items-center gap-1">
                              <i className="fa-solid fa-code"></i>
                              {langs[0]}
                            </span>
                          );
                        })()}
                        {project.stars > 0 && (
                          <span className="flex items-center gap-1 text-orange-500">
                            <i className="fa-solid fa-star"></i>
                            {project.stars}
                          </span>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
                {(groupedProjects[status] === undefined || groupedProjects[status].length === 0) && (
                  <div className="text-center text-gray-400 text-sm py-8 border-2 border-dashed border-gray-200 rounded-lg">
                    {dragOverColumn === status ? (
                      <span className="text-indigo-500">松开以移动到 {STATUS_CONFIG[status].title}</span>
                    ) : (
                      "暂无项目"
                    )}
                  </div>
                )}
              </div>
            </div>
           ))}
         </div>
          </>
        )}
      </div>

      {showMoveDialog && (
        <MoveCategoryDialog
          open={true}
          selectedCount={selectedIds.size}
          onClose={() => setShowMoveDialog(false)}
          onConfirm={async (categoryId) => {
            await onBatchMoveCategory?.(Array.from(selectedIds), categoryId);
            onSelectionChange?.(new Set());
          }}
        />
      )}

      {(showFilter || showSort) && (
        <div
          className="fixed inset-0 z-0"
          onClick={() => { setShowFilter(false); setShowSort(false); }}
        />
      )}

      {draggingProject && dragPos && (
        <div
          className="fixed pointer-events-none z-50 bg-white rounded-lg border border-indigo-300 shadow-lg p-4 w-64 opacity-90"
          style={{
            left: dragPos.x - draggingProject.offsetX,
            top: dragPos.y - draggingProject.offsetY,
          }}
        >
          <h4 className="font-medium text-sm truncate">{draggingProject.project.name}</h4>
          <p className="text-xs text-gray-500">{draggingProject.project.source}</p>
          <p className="text-xs text-gray-600 mt-2 line-clamp-2">
            {draggingProject.project.description || "-"}
          </p>
        </div>
      )}
    </div>
  );
}
