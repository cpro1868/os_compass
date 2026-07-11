import { useState, useMemo, useRef, useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { Project, Category } from "../types";
import { parseLanguages } from "../types";
import { getCategories } from "../api";
import { MoveCategoryDialog } from "./MoveCategoryDialog";
import { EmptyState } from "./EmptyState";

interface ListViewProps {
  projects: Project[];
  onProjectClick: (project: Project) => void;
  onBatchMoveCategory?: (ids: number[], categoryId: number | null) => void;
  onBatchArchive?: (ids: number[]) => void;
  selectedIds: Set<number>;
  onSelectionChange: (ids: Set<number>) => void;
}

const STATUS_CONFIG: Record<string, { emoji: string; label: string; class: string }> = {
  TO_EXPLORE: { emoji: "💡", label: "待探索", class: "bg-blue-100 text-blue-700" },
  DIVING: { emoji: "🔬", label: "深度研究中", class: "bg-yellow-100 text-yellow-700" },
  IN_USE: { emoji: "✅", label: "已落地", class: "bg-green-100 text-green-700" },
  ABANDONED: { emoji: "🗑️", label: "弃用/避坑", class: "bg-gray-100 text-gray-700" },
};

export function ListView({ projects, onProjectClick, onBatchMoveCategory, onBatchArchive, selectedIds, onSelectionChange }: ListViewProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState("");
  const [filterCategory, setFilterCategory] = useState("");
  const [filterStatus, setFilterStatus] = useState("");
  const [filterLanguages, setFilterLanguages] = useState<string[]>([]);
  const [showLangDropdown, setShowLangDropdown] = useState(false);
  const [openMenuId, setOpenMenuId] = useState<number | null>(null);
  const [showMoveDialog, setShowMoveDialog] = useState(false);
  const [columnWidths, setColumnWidths] = useState<Record<string, number>>({});
  const [categories, setCategories] = useState<Category[]>([]);
  const tableRef = useRef<HTMLDivElement>(null);

  const COMMON_LANGUAGES = [
    "JavaScript", "TypeScript", "Python", "Java", "Go", "Rust", "C++", "C",
    "C#", "PHP", "Ruby", "Swift", "Kotlin", "Dart", "Scala", "R",
    "HTML", "CSS", "Shell", "Lua", "Elixir", "Haskell", "Clojure",
    "Vue", "Svelte", "Deno", "Bun",
  ];

  useEffect(() => {
    getCategories().then(setCategories).catch(() => {});
  }, []);

  const handleResizeStart = useCallback((colKey: string, e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const startX = e.clientX;
    const startWidth = columnWidths[colKey] || 0;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const diff = moveEvent.clientX - startX;
      const newWidth = Math.max(50, startWidth + diff);
      setColumnWidths(prev => ({ ...prev, [colKey]: newWidth }));
    };

    const handleMouseUp = () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, [columnWidths]);

  const getColWidth = (key: string, defaultWidth: number) => {
    return columnWidths[key] || defaultWidth;
  };

  const allLanguages = useMemo(() => {
    const langs = new Set<string>(COMMON_LANGUAGES);
    projects.forEach((p) => {
      parseLanguages(p.languages).forEach((l) => langs.add(l));
    });
    return Array.from(langs).sort();
  }, [projects]);

  const buildCategoryPath = (catId: number | null): string => {
    if (!catId) return "未分类";
    const parts: string[] = [];
    let current = categories.find((c) => c.id === catId);
    const visited = new Set<number>();
    while (current && !visited.has(current.id)) {
      visited.add(current.id);
      parts.unshift(current.name);
      current = current.parent_id ? categories.find((c) => c.id === current!.parent_id) : undefined;
    }
    return parts.join(" / ") || "未分类";
  };

  const getCategoryDepth = (cat: Category): number => {
    let depth = 0;
    let current: Category | undefined = cat;
    const visited = new Set<number>();
    while (current?.parent_id && !visited.has(current.id)) {
      visited.add(current.id);
      current = categories.find((c) => c.id === current!.parent_id);
      if (!current) break;
      depth++;
    }
    return depth;
  };

  const categoryOptions = useMemo(() => {
    const result: { id: number; label: string; depth: number }[] = [];
    categories.forEach((cat) => {
      const depth = getCategoryDepth(cat);
      const indent = "　".repeat(depth);
      const prefix = depth > 0 ? "├ " : "";
      result.push({ id: cat.id, label: `${indent}${prefix}${cat.name}`, depth });
    });
    result.sort((a, b) => {
      const pathA = buildCategoryPath(a.id);
      const pathB = buildCategoryPath(b.id);
      return pathA.localeCompare(pathB, "zh-CN");
    });
    return result;
  }, [categories]);

  const filteredProjects = useMemo(() => {
    return projects.filter((p) => {
      if (searchQuery && !p.name.toLowerCase().includes(searchQuery.toLowerCase())) {
        return false;
      }
      if (filterStatus && p.lifecycle_status !== filterStatus) {
        return false;
      }
      if (filterCategory && p.category_id !== Number(filterCategory)) {
        return false;
      }
      if (filterLanguages.length > 0) {
        const projectLangs = parseLanguages(p.languages);
        if (projectLangs.length === 0) return false;
        const hasLang = filterLanguages.some((l) => projectLangs.includes(l));
        if (!hasLang) return false;
      }
      return true;
    });
  }, [projects, searchQuery, filterStatus, filterCategory, filterLanguages]);

  const toggleSelect = (id: number, e?: React.MouseEvent) => {
    e?.stopPropagation();
    const newSet = new Set(selectedIds);
    if (newSet.has(id)) {
      newSet.delete(id);
    } else {
      newSet.add(id);
    }
    onSelectionChange(newSet);
  };

  const toggleSelectAll = () => {
    if (selectedIds.size === filteredProjects.length) {
      onSelectionChange(new Set());
    } else {
      onSelectionChange(new Set(filteredProjects.map((p) => p.id)));
    }
  };

  const toggleLang = (lang: string) => {
    if (filterLanguages.includes(lang)) {
      setFilterLanguages(filterLanguages.filter((l) => l !== lang));
    } else {
      setFilterLanguages([...filterLanguages, lang]);
    }
  };

  const handleMoveCategory = (categoryId: number | null) => {
    if (onBatchMoveCategory) {
      onBatchMoveCategory(Array.from(selectedIds), categoryId);
    }
    onSelectionChange(new Set());
  };

  const handleBatchArchive = () => {
    if (!onBatchArchive) return;
    if (!confirm(`确定要归档选中的 ${selectedIds.size} 个项目吗？`)) return;
    onBatchArchive(Array.from(selectedIds));
    onSelectionChange(new Set());
  };

  const handleExport = () => {
    const selected = projects.filter((p) => selectedIds.has(p.id));
    const exportData = selected.map((p) => ({
      id: p.id,
      name: p.name,
      url: p.url,
      source: p.source,
      description: p.description,
      languages: parseLanguages(p.languages),
      stars: p.stars,
      forks: p.forks,
      license: p.license,
      category_id: p.category_id,
      lifecycle_status: p.lifecycle_status,
      health_score: p.health_score,
      ai_summary: p.ai_summary,
      ai_use_cases: p.ai_use_cases,
      ai_risks: p.ai_risks,
      ai_dependencies: p.ai_dependencies,
      created_at: p.created_at,
      updated_at: p.updated_at,
    }));
    const json = JSON.stringify(exportData, null, 2);
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `projects_export_${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const statusText = (status: string) => {
    const config = STATUS_CONFIG[status] || STATUS_CONFIG.TO_EXPLORE;
    return `${config.emoji} ${config.label}`;
  };

  return (
    <div className="flex flex-col h-full">
      {/* 顶部栏 */}
      <header className="bg-white border-b border-gray-200 px-6 py-4">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-4">
            <h1 className="text-xl font-bold">全部项目</h1>
            <span className="text-sm text-gray-500">共 {filteredProjects.length} 个</span>
          </div>
        </div>

        <div className="flex items-center gap-3">
          {/* 搜索框 */}
          <div className="flex-1 max-w-md relative">
            <i className="fa-solid fa-magnifying-glass absolute left-3 top-1/2 -translate-y-1/2 text-gray-400"></i>
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t("list.search")}
              className="w-full pl-10 pr-4 py-2 border rounded-lg text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            />
          </div>

          {/* 分类筛选 */}
          <select
            value={filterCategory}
            onChange={(e) => setFilterCategory(e.target.value)}
            className="px-3 py-2 border rounded-lg text-sm"
          >
            <option value="">全部分类</option>
            {categoryOptions.map((cat) => (
              <option key={cat.id} value={cat.id}>{cat.label}</option>
            ))}
          </select>

          {/* 语言筛选 */}
          <div className="relative">
            <button
              onClick={() => {
                setShowLangDropdown(!showLangDropdown);
              }}
              className="px-3 py-2 border rounded-lg text-sm flex items-center gap-2 hover:bg-gray-50"
            >
              <span>语言</span>
              {filterLanguages.length > 0 && (
                <span className="px-1.5 py-0.5 bg-blue-100 text-blue-600 text-xs rounded">
                  {filterLanguages.length}
                </span>
              )}
              <i className="fa-solid fa-chevron-down text-xs"></i>
            </button>
            {showLangDropdown && (
              <div className="absolute z-10 mt-1 bg-white border rounded-lg shadow-lg p-2 min-w-[160px] right-0 max-h-64 overflow-auto">
                <label
                  className="flex items-center gap-2 px-2 py-1.5 hover:bg-gray-50 rounded text-sm cursor-pointer border-b"
                >
                  <input
                    type="checkbox"
                    checked={filterLanguages.length === 0}
                    onChange={() => setFilterLanguages([])}
                    className="rounded"
                  />
                  <span className="font-medium">全部语言</span>
                </label>
                {allLanguages.map((lang) => (
                  <label
                    key={lang}
                    className="flex items-center gap-2 px-2 py-1.5 hover:bg-gray-50 rounded text-sm cursor-pointer"
                  >
                    <input
                      type="checkbox"
                      checked={filterLanguages.includes(lang)}
                      onChange={() => toggleLang(lang)}
                      className="rounded"
                    />
                    <span>{lang}</span>
                  </label>
                ))}
              </div>
            )}
          </div>

          {/* 状态筛选 */}
          <select
            value={filterStatus}
            onChange={(e) => setFilterStatus(e.target.value)}
            className="px-3 py-2 border rounded-lg text-sm"
          >
            <option value="">全部状态</option>
            <option value="TO_EXPLORE">💡 待探索</option>
            <option value="DIVING">🔬 深度研究中</option>
            <option value="IN_USE">✅ 已落地</option>
            <option value="ABANDONED">🗑️ 弃用/避坑</option>
          </select>
        </div>
      </header>

      {/* 批量操作栏 */}
      {selectedIds.size > 0 && (
        <div className="bg-blue-600 text-white px-6 py-3 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <span>已选中 {selectedIds.size} 项</span>
            <button
              onClick={toggleSelectAll}
              className="text-sm underline hover:text-blue-100"
            >
              全选
            </button>
            <button
              onClick={() => onSelectionChange(new Set())}
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
              <i className="fa-solid fa-box-archive"></i>批量归档
            </button>
            <button
              onClick={handleExport}
              className="px-3 py-1.5 bg-white/20 hover:bg-white/30 rounded-lg text-sm flex items-center gap-2"
            >
              <i className="fa-solid fa-download"></i>导出
            </button>
          </div>
        </div>
      )}

      {/* 表格 */}
      <div className="flex-1 overflow-auto" ref={tableRef}>
        <table className="w-full table-fixed" style={{ tableLayout: 'fixed' }}>
          <colgroup>
            <col style={{ width: getColWidth('checkbox', 40) }} />
            <col style={{ width: getColWidth('name', 250) }} />
            <col style={{ width: getColWidth('category', 100) }} />
            <col style={{ width: getColWidth('stars', 80) }} />
            <col style={{ width: getColWidth('language', 120) }} />
            <col style={{ width: getColWidth('status', 100) }} />
            <col style={{ width: getColWidth('updated', 100) }} />
            <col style={{ width: getColWidth('actions', 80) }} />
          </colgroup>
          <thead className="bg-gray-50 sticky top-0">
            <tr className="text-left text-xs text-gray-500 border-b">
              <th className="px-2 py-2">
                <input
                  type="checkbox"
                  checked={selectedIds.size === filteredProjects.length && filteredProjects.length > 0}
                  onChange={toggleSelectAll}
                  className="rounded"
                />
              </th>
              <th className="px-2 py-2 relative group">
                <div className="flex items-center">
                  <span>项目名称</span>
                  <div
                    className="absolute right-0 top-0 bottom-0 w-2 cursor-col-resize hover:bg-blue-400/30"
                    onMouseDown={(e) => handleResizeStart('name', e)}
                  />
                </div>
              </th>
              <th className="px-2 py-2 relative group">
                分类
                <div
                  className="absolute right-0 top-0 bottom-0 w-2 cursor-col-resize hover:bg-blue-400/30"
                  onMouseDown={(e) => handleResizeStart('category', e)}
                />
              </th>
              <th className="px-2 py-2 relative group">
                Stars
                <div
                  className="absolute right-0 top-0 bottom-0 w-2 cursor-col-resize hover:bg-blue-400/30"
                  onMouseDown={(e) => handleResizeStart('stars', e)}
                />
              </th>
              <th className="px-2 py-2 relative group">
                语言
                <div
                  className="absolute right-0 top-0 bottom-0 w-2 cursor-col-resize hover:bg-blue-400/30"
                  onMouseDown={(e) => handleResizeStart('language', e)}
                />
              </th>
              <th className="px-2 py-2 relative group">
                状态
                <div
                  className="absolute right-0 top-0 bottom-0 w-2 cursor-col-resize hover:bg-blue-400/30"
                  onMouseDown={(e) => handleResizeStart('status', e)}
                />
              </th>
              <th className="px-2 py-2 relative group">
                更新时间
                <div
                  className="absolute right-0 top-0 bottom-0 w-2 cursor-col-resize hover:bg-blue-400/30"
                  onMouseDown={(e) => handleResizeStart('updated', e)}
                />
              </th>
              <th className="px-2 py-2">操作</th>
            </tr>
          </thead>
          <tbody className="bg-white divide-y divide-gray-100">
            {filteredProjects.length === 0 ? (
              <tr>
                <td colSpan={8} className="px-6 py-12">
                  {projects.length === 0 ? (
                    <EmptyState
                      icon="fa-inbox"
                      title="还没有项目"
                      description="导入你的第一个开源项目，开始管理你的开源资产"
                      actionLabel="导入项目"
                      onAction={() => window.dispatchEvent(new CustomEvent("openImport"))}
                    />
                  ) : (
                    <EmptyState
                      icon="fa-magnifying-glass"
                      title="没有找到匹配的项目"
                      description="尝试调整搜索关键词或筛选条件"
                    />
                  )}
                </td>
              </tr>
            ) : (
              filteredProjects.map((project) => (
                <tr
                  key={project.id}
                  className={`hover:bg-gray-50 ${selectedIds.has(project.id) ? "bg-blue-50" : ""}`}
                >
                  <td className="px-3 py-2">
                    <input
                      type="checkbox"
                      checked={selectedIds.has(project.id)}
                      onChange={(e) => toggleSelect(project.id, e as any)}
                      className="rounded"
                    />
                  </td>
                  <td className="px-3 py-2">
                    <div className="flex items-center gap-2">
                      <div className="w-6 h-6 bg-orange-100 rounded flex items-center justify-center text-orange-600">
                        <i className="fa-solid fa-code text-xs"></i>
                      </div>
                      <div className="min-w-0">
                        <button
                          onClick={() => onProjectClick(project)}
                          className="font-medium text-blue-600 hover:underline text-sm truncate block"
                        >
                          {project.name}
                        </button>
                        <p className="text-xs text-gray-400 truncate">{project.url}</p>
                      </div>
                    </div>
                  </td>
                  <td className="px-3 py-2 text-xs text-gray-600">
                    <span className="truncate block" title={buildCategoryPath(project.category_id ?? null)}>
                      {buildCategoryPath(project.category_id ?? null)}
                    </span>
                  </td>
                  <td className="px-3 py-2 text-xs">{project.stars > 0 ? `★ ${project.stars}` : "-"}</td>
                  <td className="px-3 py-2">
                    {(() => {
                      const langs = parseLanguages(project.languages);
                      if (langs.length === 0) return <span className="text-xs text-gray-400">-</span>;
                      return (
                        <div className="flex flex-wrap gap-1">
                          {langs.slice(0, 3).map((lang) => (
                            <span key={lang} className="px-1.5 py-0.5 bg-blue-100 text-blue-700 text-xs rounded">
                              {lang}
                            </span>
                          ))}
                        </div>
                      );
                    })()}
                  </td>
                  <td className="px-3 py-2">
                    <span className={`px-2 py-0.5 text-xs rounded ${STATUS_CONFIG[project.lifecycle_status]?.class || ""}`}>
                      {statusText(project.lifecycle_status)}
                    </span>
                  </td>
                  <td className="px-3 py-2 text-xs text-gray-400">
                    {new Date(project.updated_at).toLocaleDateString()}
                  </td>
                  <td className="px-6 py-3">
                    <div className="relative" onMouseLeave={() => setOpenMenuId(null)}>
                      <button
                        onClick={() => setOpenMenuId(openMenuId === project.id ? null : project.id)}
                        className="text-gray-400 hover:text-gray-600"
                      >
                        <i className="fa-solid fa-ellipsis"></i>
                      </button>
                      {openMenuId === project.id && (
                        <div className="absolute right-0 mt-1 bg-white border rounded-lg shadow-lg py-1 min-w-[120px] z-10">
                          <button
                            onClick={() => { onProjectClick(project); setOpenMenuId(null); }}
                            className="w-full text-left px-4 py-2 text-sm hover:bg-gray-50 flex items-center gap-2"
                          >
                            <i className="fa-solid fa-pen"></i>编辑
                          </button>
                          <button
                            onClick={() => {
                              if (confirm("确定要归档此项目吗？")) {
                                invoke("archive_project", { id: project.id }).catch(console.error);
                              }
                              setOpenMenuId(null);
                            }}
                            className="w-full text-left px-4 py-2 text-sm hover:bg-gray-50 flex items-center gap-2"
                          >
                            <i className="fa-solid fa-box-archive"></i>归档
                          </button>
                          <button
                            onClick={() => {
                              if (confirm("确定要删除此项目吗？此操作不可恢复！")) {
                                invoke("permanent_delete_project", { id: project.id }).catch(console.error);
                              }
                              setOpenMenuId(null);
                            }}
                            className="w-full text-left px-4 py-2 text-sm hover:bg-gray-50 text-red-600 flex items-center gap-2"
                          >
                            <i className="fa-solid fa-trash"></i>删除
                          </button>
                        </div>
                      )}
                    </div>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
      <style>{`
        .cursor-col-resize {
          cursor: col-resize;
          user-select: none;
        }
      `}</style>

      <MoveCategoryDialog
        open={showMoveDialog}
        selectedCount={selectedIds.size}
        onClose={() => setShowMoveDialog(false)}
        onConfirm={handleMoveCategory}
      />
    </div>
  );
}
