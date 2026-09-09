import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import type { Category } from "../types";
import * as api from "../api";
import { tagApi, type Tag } from "../api/tag";
import { EmptyState } from "./EmptyState";
import { ImportCategoryDialog } from "./ImportCategoryDialog";

interface CategoryNode extends Category {
  children?: CategoryNode[];
  projectCount?: number;
}

type TabType = "category" | "tag";
type TagSource = "ai" | "user" | "preset";

interface CategoryManagerProps {
  onBack: () => void;
}

export function CategoryManager({ onBack }: CategoryManagerProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<TabType>("category");
  const [categories, setCategories] = useState<Category[]>([]);
  const [loadingCategories, setLoadingCategories] = useState(true);
  const [expandedIds, setExpandedIds] = useState<Set<number>>(new Set([1]));
  const [editingCategory, setEditingCategory] = useState<Category | null>(null);
  const [showNewForm, setShowNewForm] = useState(false);
  const [lastSelectedId, setLastSelectedId] = useState<number | null>(null);
  const [draggingId, setDraggingId] = useState<number | null>(null);
  const [dragOverId, setDragOverId] = useState<number | null>(null);
  const [dragOverPos, setDragOverPos] = useState<"before" | "inside" | "after" | null>(null);
  const [showImportDialog, setShowImportDialog] = useState(false);
  const [formData, setFormData] = useState({
    name: "",
    parent_id: null as number | null,
    sort_order: 0,
    explain: "",
  });

  // Tag state
  const [tags, setTags] = useState<Tag[]>([]);
  const [loadingTags, setLoadingTags] = useState(true);
  const [searchQuery, setSearchQuery] = useState("");
  const [filterSource, setFilterSource] = useState<TagSource | "">("");
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newTags, setNewTags] = useState("");
  const [newTagSource, setNewTagSource] = useState<TagSource>("user");
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    loadCategories();
    loadTags();
  }, []);

  const loadCategories = async () => {
    setLoadingCategories(true);
    try {
      const cats = await api.getCategories();
      setCategories(cats);
      setExpandedIds(new Set([1]));
    } catch (e) {
      console.error(e);
    } finally {
      setLoadingCategories(false);
    }
  };

  const loadTags = async () => {
    setLoadingTags(true);
    try {
      const allTags = await tagApi.getAll();
      setTags(allTags);
    } catch (e) {
      console.error("Failed to load tags:", e);
    } finally {
      setLoadingTags(false);
    }
  };

  const handleCreateTags = async () => {
    const tagNames = newTags.split("\n").map((t) => t.trim()).filter(Boolean);
    if (tagNames.length === 0) return;

    setCreating(true);
    try {
      for (const name of tagNames) {
        await tagApi.create({ name, source: newTagSource });
      }
      setNewTags("");
      setShowCreateModal(false);
      loadTags();
    } catch (e) {
      console.error("Failed to create tags:", e);
    } finally {
      setCreating(false);
    }
  };

  const handleDeleteTag = async (id: number) => {
    try {
      await tagApi.delete(id);
      loadTags();
    } catch (e) {
      console.error("Failed to delete tag:", e);
    }
  };

  const filteredTags = tags.filter((tag) => {
    if (searchQuery && !tag.name.toLowerCase().includes(searchQuery.toLowerCase())) {
      return false;
    }
    if (filterSource && tag.source !== filterSource) {
      return false;
    }
    return true;
  });

  const aiTags = filteredTags.filter((t) => t.source === "ai");
  const userTags = filteredTags.filter((t) => t.source === "user");
  const presetTags = filteredTags.filter((t) => t.source === "preset");

  const sourceColors: Record<TagSource, { bg: string; text: string; icon: string }> = {
    ai: { bg: "bg-purple-100 dark:bg-purple-900", text: "text-purple-700 dark:text-purple-300", icon: "fa-solid fa-robot" },
    user: { bg: "bg-pink-100 dark:bg-pink-900", text: "text-pink-700 dark:text-pink-300", icon: "fa-solid fa-user" },
    preset: { bg: "bg-gray-100 dark:bg-gray-700", text: "text-gray-700 dark:text-gray-300", icon: "fa-solid fa-star" },
  };

  const sourceNames: Record<TagSource, string> = {
    ai: t("tag.aiSource"),
    user: t("tag.userSource"),
    preset: t("tag.presetSource"),
  };

  const sourceDescs: Record<TagSource, string> = {
    ai: t("tag.aiSourceDesc"),
    user: t("tag.userSourceDesc"),
    preset: t("tag.presetSourceDesc"),
  };

  const buildTree = (cats: Category[], parentId: number | null = null): CategoryNode[] => {
    return cats
      .filter((c) => c.parent_id === parentId)
      .sort((a, b) => (a.sort_order || 0) - (b.sort_order || 0))
      .map((c) => ({
        ...c,
        children: buildTree(cats, c.id),
      }));
  };

  const categoryTree = buildTree(categories);

  const toggleExpand = (id: number) => {
    const newSet = new Set(expandedIds);
    if (newSet.has(id)) {
      newSet.delete(id);
    } else {
      newSet.add(id);
    }
    setExpandedIds(newSet);
  };

  const handleSaveCategory = async () => {
    try {
      if (editingCategory) {
        await api.updateCategory({
          ...editingCategory,
          name: formData.name,
          parent_id: formData.parent_id,
          sort_order: formData.sort_order,
        });
      } else {
        await api.createCategory({
          name: formData.name,
          parent_id: formData.parent_id,
          sort_order: formData.sort_order,
        });
      }
      setEditingCategory(null);
      setShowNewForm(false);
      setFormData({ name: "", parent_id: null, sort_order: 0, explain: "" });
      loadCategories();
    } catch (e) {
      console.error(e);
    }
  };

  const handleDeleteCategory = async (id: number) => {
    if (confirm("确定要删除此分类吗？")) {
      try {
        await api.deleteCategory(id);
        loadCategories();
      } catch (e) {
        console.error(e);
      }
    }
  };

  const openEditCategory = (cat: Category) => {
    setEditingCategory(cat);
    setFormData({
      name: cat.name,
      parent_id: cat.parent_id,
      sort_order: cat.sort_order || 0,
      explain: cat.explain || "",
    });
  };

  const openNewCategory = (parentId: number | null = null) => {
    setEditingCategory(null);
    setFormData({ name: "", parent_id: parentId, sort_order: 0, explain: "" });
    setShowNewForm(true);
  };

  const handleCategoryClick = (cat: Category) => {
    setLastSelectedId(cat.id);
  };

  const isDescendant = (catId: number, ancestorId: number, cats: Category[]): boolean => {
    let current = cats.find((c) => c.id === catId);
    const visited = new Set<number>();
    while (current && current.parent_id && !visited.has(current.id)) {
      visited.add(current.id);
      if (current.parent_id === ancestorId) return true;
      current = cats.find((c) => c.id === current!.parent_id);
    }
    return false;
  };

  const handleDragStart = (e: React.DragEvent, cat: Category) => {
    if (cat.is_system) { e.preventDefault(); return; }
    setDraggingId(cat.id);
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", String(cat.id));
  };

  const handleDragOver = (e: React.DragEvent, cat: Category) => {
    if (!draggingId || cat.id === draggingId) return;
    if (cat.is_system && draggingId !== null) return;
    if (isDescendant(cat.id, draggingId, categories)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const y = e.clientY - rect.top;
    const h = rect.height;
    if (y < h * 0.25) {
      setDragOverId(cat.id);
      setDragOverPos("before");
    } else if (y > h * 0.75) {
      setDragOverId(cat.id);
      setDragOverPos("after");
    } else {
      setDragOverId(cat.id);
      setDragOverPos("inside");
    }
  };

  const handleDragLeave = (_e: React.DragEvent, cat: Category) => {
    if (dragOverId === cat.id) {
      setDragOverId(null);
      setDragOverPos(null);
    }
  };

  const handleDrop = async (e: React.DragEvent, targetCat: Category) => {
    e.preventDefault();
    e.stopPropagation();
    if (!draggingId || dragOverPos === null) return;
    const dragCat = categories.find((c) => c.id === draggingId);
    if (!dragCat) return;
    if (isDescendant(targetCat.id, draggingId, categories)) return;

    let newParentId: number | null;
    let newSortOrder: number;

    if (dragOverPos === "inside") {
      newParentId = targetCat.id;
      const siblings = categories.filter((c) => c.parent_id === targetCat.id && c.id !== draggingId);
      newSortOrder = siblings.length;
    } else {
      newParentId = targetCat.parent_id;
      const siblings = categories.filter((c) => c.parent_id === newParentId && c.id !== draggingId);
      const targetIdx = siblings.findIndex((c) => c.id === targetCat.id);
      if (dragOverPos === "before") {
        newSortOrder = targetIdx >= 0 ? siblings[targetIdx].sort_order : siblings.length;
        siblings.forEach((s, i) => { if (i >= (targetIdx >= 0 ? targetIdx : siblings.length)) { updateCategoryOrder(s, newParentId, i + 1); } });
      } else {
        newSortOrder = targetIdx >= 0 ? siblings[targetIdx].sort_order + 1 : siblings.length;
        siblings.forEach((s, i) => { if (i > targetIdx) { updateCategoryOrder(s, newParentId, i + 1); } });
      }
    }

    try {
      await api.updateCategory({
        ...dragCat,
        parent_id: newParentId,
        sort_order: newSortOrder,
      });
      await loadCategories();
    } catch (err) {
      console.error("拖拽排序失败:", err);
    } finally {
      setDraggingId(null);
      setDragOverId(null);
      setDragOverPos(null);
    }
  };

  const updateCategoryOrder = async (cat: Category, parentId: number | null, sortOrder: number) => {
    try {
      await api.updateCategory({ ...cat, parent_id: parentId, sort_order: sortOrder });
    } catch (err) {
      console.error("更新分类排序失败:", err);
    }
  };

  const handleDragEnd = () => {
    setDraggingId(null);
    setDragOverId(null);
    setDragOverPos(null);
  };

  const renderParentOptions = (cats: Category[], level: number = 0): React.ReactNode[] => {
    const options: React.ReactNode[] = [];
    cats
      .filter((c) => !c.is_system)
      .sort((a, b) => (a.sort_order || 0) - (b.sort_order || 0))
      .forEach((cat) => {
        const prefix = "　".repeat(level).replace(/　/g, "\u00A0\u00A0\u00A0\u00A0");
        options.push(
          <option key={cat.id} value={cat.id}>
            {prefix}{cat.name}
          </option>
        );
        options.push(...renderParentOptions(categories.filter((c) => c.parent_id === cat.id), level + 1));
      });
    return options;
  };

  const renderCategory = (cat: CategoryNode, level: number = 0) => {
    const hasChildren = cat.children && cat.children.length > 0;
    const isExpanded = expandedIds.has(cat.id);
    const isSystem = cat.is_system;
    const marginLeft = level * 24;
    const isDragging = draggingId === cat.id;
    const isDragOver = dragOverId === cat.id;
    const dropIndicator = isDragOver ? dragOverPos : null;

    return (
      <div key={cat.id}>
        <div
          draggable={!isSystem}
          onDragStart={(e) => handleDragStart(e, cat)}
          onDragOver={(e) => handleDragOver(e, cat)}
          onDragLeave={(e) => handleDragLeave(e, cat)}
          onDrop={(e) => handleDrop(e, cat)}
          onDragEnd={handleDragEnd}
          className={`flex items-center justify-between p-3 rounded-lg mb-1 transition-colors relative ${
            isDragging ? "opacity-40" : ""
          } ${
            lastSelectedId === cat.id ? "bg-blue-100 dark:bg-blue-900 border border-blue-300 dark:border-blue-700" : "bg-gray-50 dark:bg-gray-700 hover:bg-gray-100 dark:hover:bg-gray-600"
          } ${isDragOver && dropIndicator === "inside" ? "ring-2 ring-indigo-400 bg-indigo-50 dark:bg-indigo-950" : ""} ${
            !isSystem ? "cursor-grab active:cursor-grabbing" : ""
          }`}
          style={{ marginLeft }}
          onClick={() => handleCategoryClick(cat)}
        >
          {isDragOver && dropIndicator === "before" && (
            <div className="absolute left-0 right-0 -top-0.5 h-0.5 bg-indigo-500 rounded-full"></div>
          )}
          {isDragOver && dropIndicator === "after" && (
            <div className="absolute left-0 right-0 -bottom-0.5 h-0.5 bg-indigo-500 rounded-full"></div>
          )}
          <div className="flex items-center gap-2">
            {!isSystem && (
              <i className="fa-solid fa-grip-vertical text-gray-300 text-xs"></i>
            )}
            <button
              onClick={(e) => {
                e.stopPropagation();
                if (hasChildren) toggleExpand(cat.id);
              }}
              className="w-6 h-6 flex items-center justify-center text-gray-500 dark:text-gray-400 hover:text-blue-600 dark:hover:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-950 rounded"
              title={hasChildren ? (isExpanded ? "收起" : "展开") : ""}
            >
              {hasChildren ? (
                <i className={`fa-solid ${isExpanded ? "fa-chevron-down" : "fa-chevron-right"}`}></i>
              ) : (
                <i className="fa-solid fa-circle text-xs text-gray-300"></i>
              )}
            </button>
            <i className={`fa-solid fa-folder ${hasChildren ? "text-yellow-500" : "text-yellow-300"}`}></i>
            <span className={`font-medium ${isSystem ? "text-gray-600 dark:text-gray-400" : "text-gray-800 dark:text-gray-200"}`}>{cat.name}</span>
            {isSystem && <span className="text-xs text-gray-400 dark:text-gray-500 bg-gray-200 dark:bg-gray-700 px-2 py-0.5 rounded">{t("category.system")}</span>}
            {hasChildren && (
              <span className="text-xs text-gray-400 dark:text-gray-500">({cat.children!.length})</span>
            )}
          </div>
          <div className="flex items-center gap-1">
            {!isSystem && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  openNewCategory(cat.id);
                }}
                className="p-1.5 text-gray-400 dark:text-gray-500 hover:text-green-600 dark:hover:text-green-400 hover:bg-green-50 dark:hover:bg-green-950 rounded"
                title={t("category.addChild")}
              >
                <i className="fa-solid fa-plus"></i>
              </button>
            )}
            {!isSystem && (
              <>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    openEditCategory(cat);
                  }}
                  className="p-1.5 text-gray-400 dark:text-gray-500 hover:text-blue-600 dark:hover:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-950 rounded"
                  title={t("category.edit")}
                >
                  <i className="fa-solid fa-pen-to-square"></i>
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDeleteCategory(cat.id);
                  }}
                  className="p-1.5 text-gray-400 dark:text-gray-500 hover:text-red-600 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-950 rounded"
                  title={t("category.delete")}
                >
                  <i className="fa-solid fa-trash"></i>
                </button>
              </>
            )}
          </div>
        </div>
        {hasChildren && isExpanded && (
          <div>
            {cat.children!.map((child) => renderCategory(child, level + 1))}
          </div>
        )}
      </div>
    );
  };

  const renderCategoryItem = (cat: CategoryNode, level: number = 0) => (
    <div key={cat.id}>
      <div
        className="flex items-center gap-2 px-3 py-2 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg group"
        style={{ marginLeft: level * 32 }}
      >
        {cat.children && cat.children.length > 0 && (
          <button
            onClick={() => {
              const newSet = new Set(expandedIds);
              if (newSet.has(cat.id)) {
                newSet.delete(cat.id);
              } else {
                newSet.add(cat.id);
              }
              setExpandedIds(newSet);
            }}
            className="w-5 h-5 flex items-center justify-center text-gray-400"
          >
            <i className={`fa-solid fa-chevron-right text-xs transition-transform ${expandedIds.has(cat.id) ? "rotate-90" : ""}`}></i>
          </button>
        )}
        {(level === 0 || !cat.children || cat.children.length === 0) && <div className="w-5"></div>}

        <i className={`fa-solid fa-folder ${cat.children && cat.children.length > 0 ? "text-yellow-500" : "text-yellow-300"}`}></i>
        <span className={`flex-1 font-medium ${cat.is_system ? "text-gray-500 dark:text-gray-400" : "dark:text-gray-200"}`}>{cat.name}</span>
        {cat.is_system && (
          <span className="px-2 py-0.5 text-xs bg-gray-200 dark:bg-gray-600 text-gray-500 dark:text-gray-400 rounded">
            {t("category.system") || "系统"}
          </span>
        )}
        {cat.project_count && cat.project_count > 0 && (
          <span className="text-xs text-gray-400 dark:text-gray-500">({cat.project_count})</span>
        )}
        <div className="hidden group-hover:flex items-center gap-1">
          {!cat.is_system && (
            <>
              <button
                onClick={() => openEditCategory(cat)}
                className="p-1 text-gray-400 hover:text-blue-500"
              >
                <i className="fa-solid fa-pen text-xs"></i>
              </button>
              <button
                onClick={() => handleDeleteCategory(cat.id)}
                className="p-1 text-gray-400 hover:text-red-500"
              >
                <i className="fa-solid fa-trash text-xs"></i>
              </button>
              <button
                onClick={() => openNewCategory(cat.id)}
                className="p-1 text-gray-400 hover:text-green-500"
              >
                <i className="fa-solid fa-plus text-xs"></i>
              </button>
            </>
          )}
        </div>
      </div>
      {expandedIds.has(cat.id) && cat.children && (
        <div className="mt-1 space-y-1">
          {cat.children.map((child) => renderCategoryItem(child, level + 1))}
        </div>
      )}
    </div>
  );

  // ==========================================
  // 标签 Tab 内容渲染
  // ==========================================
  const renderTagSection = () => (
    <div className="space-y-4">
      <div className="flex gap-4">
        <div className="flex-1 relative">
          <i className="fa-solid fa-search absolute left-3 top-1/2 -translate-y-1/2 text-gray-400"></i>
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={`${t("tag.name")}...`}
            className="w-full pl-10 pr-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
          />
        </div>
        <select
          value={filterSource}
          onChange={(e) => setFilterSource(e.target.value as TagSource | "")}
          className="px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
        >
          <option value="">{t("kanban.all")} {t("tag.source")}</option>
          <option value="ai">{t("tag.aiSource")}</option>
          <option value="user">{t("tag.userSource")}</option>
          <option value="preset">{t("tag.presetSource")}</option>
        </select>
        <button
          onClick={() => setShowCreateModal(true)}
          className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 flex items-center gap-2"
        >
          <i className="fa-solid fa-plus"></i>
          {t("tag.create") || "新建标签"}
        </button>
      </div>

      {loadingTags ? (
        <div className="text-center py-12 text-gray-500">
          <i className="fa-solid fa-spinner fa-spin text-2xl"></i>
          <p className="mt-2">{t("common.loading")}</p>
        </div>
      ) : (
        <div className="space-y-3">
          {(!filterSource || filterSource === "ai") && (
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4">
              <h3 className="font-medium text-gray-700 dark:text-gray-300 mb-3">
                <i className={`${sourceColors.ai.icon} text-blue-500 dark:text-blue-400 mr-2`}></i>
                {sourceNames.ai}
                <span className="text-sm font-normal text-gray-400 dark:text-gray-500 ml-2">（{sourceDescs.ai}）</span>
              </h3>
              <div className="flex flex-wrap gap-2">
                {aiTags.length === 0 ? (
                  <span className="text-gray-400 dark:text-gray-500 text-sm">暂无 AI 提取标签</span>
                ) : (
                  aiTags.map((tag) => (
                    <span
                      key={tag.id}
                      className={`px-3 py-1.5 ${sourceColors.ai.bg} ${sourceColors.ai.text} rounded-full text-sm flex items-center gap-2`}
                    >
                      <i className={`${sourceColors.ai.icon} text-xs`}></i>
                      {tag.name}
                    </span>
                  ))
                )}
              </div>
            </div>
          )}

          {(!filterSource || filterSource === "user") && (
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4">
              <h3 className="font-medium text-gray-700 dark:text-gray-300 mb-3">
                <i className={`${sourceColors.user.icon} text-green-500 mr-2`}></i>
                {sourceNames.user}
                <span className="text-sm font-normal text-gray-400 dark:text-gray-500 ml-2">（{sourceDescs.user}）</span>
              </h3>
              <div className="flex flex-wrap gap-2">
                {userTags.length === 0 ? (
                  <span className="text-gray-400 dark:text-gray-500 text-sm">暂无用户添加标签</span>
                ) : (
                  userTags.map((tag) => (
                    <span
                      key={tag.id}
                      className={`px-3 py-1.5 ${sourceColors.user.bg} ${sourceColors.user.text} rounded-full text-sm flex items-center gap-2`}
                    >
                      {tag.name}
                      <button
                        onClick={() => handleDeleteTag(tag.id)}
                        className="hover:text-pink-900 dark:hover:text-pink-300"
                      >
                        <i className="fa-solid fa-times text-xs"></i>
                      </button>
                    </span>
                  ))
                )}
              </div>
            </div>
          )}

          {(!filterSource || filterSource === "preset") && presetTags.length > 0 && (
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4">
              <h3 className="font-medium text-gray-700 dark:text-gray-300 mb-3">
                <i className={`${sourceColors.preset.icon} text-amber-500 mr-2`}></i>
                {sourceNames.preset}
                <span className="text-sm font-normal text-gray-400 dark:text-gray-500 ml-2">（{sourceDescs.preset}）</span>
              </h3>
              <div className="flex flex-wrap gap-2">
                {presetTags.map((tag) => (
                  <span
                    key={tag.id}
                    className={`px-3 py-1.5 ${sourceColors.preset.bg} ${sourceColors.preset.text} rounded-full text-sm flex items-center gap-2`}
                  >
                    {tag.name}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );

  // ==========================================
  // Category/Tag 创建/编辑弹窗
  // ==========================================
  const renderCategoryForm = () => (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-md p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold dark:text-gray-100">
            {editingCategory ? (t("category.edit") || "编辑分类") : (t("category.create") || "新建分类")}
          </h3>
          <button
            onClick={() => {
              setShowNewForm(false);
              setEditingCategory(null);
            }}
            className="text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300"
          >
            <i className="fa-solid fa-xmark text-xl"></i>
          </button>
        </div>

        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-2 text-gray-700 dark:text-gray-300">
              {t("category.form.name") || "分类名称"}
            </label>
            <input
              type="text"
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
              autoFocus
            />
          </div>
          <div>
            <label className="block text-sm font-medium mb-2 text-gray-700 dark:text-gray-300">
              {t("category.form.parent") || "上级分类"}
            </label>
            <select
              value={formData.parent_id || ""}
              onChange={(e) =>
                setFormData({
                  ...formData,
                  parent_id: e.target.value ? Number(e.target.value) : null,
                })
              }
              className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
            >
              <option value="">{t("category.form.topLevel") || "无（顶级分类）"}</option>
              {renderParentOptions(categories.filter((c) => c.parent_id === null))}
            </select>
          </div>
          <div className="flex justify-end gap-3 pt-4 border-t dark:border-gray-700">
            <button
              onClick={() => {
                setShowNewForm(false);
                setEditingCategory(null);
              }}
              className="px-4 py-2 border rounded-lg hover:bg-gray-50 dark:border-gray-600 dark:hover:bg-gray-700"
            >
              {t("common.cancel")}
            </button>
            <button
              onClick={handleSaveCategory}
              disabled={!formData.name.trim()}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
            >
              {t("common.save")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );

  const renderTagCreateModal = () => (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-md p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold dark:text-gray-100">{t("tag.create") || "新建标签"}</h3>
          <button
            onClick={() => setShowCreateModal(false)}
            className="text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300"
          >
            <i className="fa-solid fa-xmark text-xl"></i>
          </button>
        </div>

        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-2 text-gray-700 dark:text-gray-300">
              标签名称
            </label>
            <textarea
              value={newTags}
              onChange={(e) => setNewTags(e.target.value)}
              placeholder="输入标签名称（每行一个，可批量添加）"
              rows={5}
              className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
              autoFocus
            />
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              每行一个标签，支持批量添加多个
            </p>
          </div>

          <div>
            <label className="block text-sm font-medium mb-2 text-gray-700 dark:text-gray-300">
              标签类型
            </label>
            <select
              value={newTagSource}
              onChange={(e) => setNewTagSource(e.target.value as TagSource)}
              className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
            >
              <option value="user">用户添加</option>
              <option value="preset">预设</option>
            </select>
          </div>

          <div className="flex justify-end gap-3 pt-4 border-t dark:border-gray-700">
            <button
              onClick={() => setShowCreateModal(false)}
              className="px-4 py-2 border rounded-lg hover:bg-gray-50 dark:border-gray-600 dark:hover:bg-gray-700"
            >
              {t("common.cancel")}
            </button>
            <button
              onClick={handleCreateTags}
              disabled={!newTags.trim() || creating}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
            >
              {creating ? t("common.saving") : t("common.save")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );

  // ==========================================
  // 主渲染
  // ==========================================
  return (
    <div className="flex flex-col h-full">
      <header className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4 flex items-center justify-between">
        <div className="flex items-center gap-4">
          <button
            onClick={onBack}
            className="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
          >
            <i className="fa-solid fa-arrow-left mr-2"></i>{t("common.close")}
          </button>
          <h1 className="text-xl font-bold dark:text-gray-100">{t("category.title")}</h1>
        </div>

        {/* Tab 切换 */}
        <div className="flex gap-1">
          <button
            onClick={() => setActiveTab("category")}
            className={`px-4 py-2 text-sm rounded-lg transition flex items-center gap-2 ${
              activeTab === "category"
                ? "bg-blue-600 text-white"
                : "text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700"
            }`}
          >
            <i className="fa-solid fa-folder-tree"></i>
            {t("category.title") || "分类"}
          </button>
          <button
            onClick={() => setActiveTab("tag")}
            className={`px-4 py-2 text-sm rounded-lg transition flex items-center gap-2 ${
              activeTab === "tag"
                ? "bg-blue-600 text-white"
                : "text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700"
            }`}
          >
            <i className="fa-solid fa-tag"></i>
            {t("tag.title") || "标签"}
          </button>
        </div>
      </header>

      <div className="flex-1 overflow-auto p-6">
        <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6 max-w-4xl">
          {/* 分类 Tab 内容 */}
          {activeTab === "category" && (
            <div className="space-y-4">
              <div className="flex justify-end gap-2">
                <button
                  onClick={() => setShowImportDialog(true)}
                  className="px-4 py-2 bg-emerald-600 text-white rounded-lg hover:bg-emerald-700 flex items-center gap-2"
                >
                  <i className="fa-solid fa-file-import"></i>
                  {t("category.import") || "导入分类"}
                </button>
                <button
                  onClick={() => openNewCategory(null)}
                  className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 flex items-center gap-2"
                >
                  <i className="fa-solid fa-plus"></i>
                  {t("category.create") || "新建分类"}
                </button>
              </div>

              {loadingCategories ? (
                <div className="text-center py-12 text-gray-500">
                  <i className="fa-solid fa-spinner fa-spin text-2xl"></i>
                  <p className="mt-2">{t("common.loading")}</p>
                </div>
              ) : categoryTree.length === 0 ? (
                <EmptyState
                  icon="fa-folder-tree"
                  title={t("category.empty")}
                  description={t("category.emptyDesc")}
                  actionLabel={t("category.createFirst")}
                  onAction={() => openNewCategory(null)}
                />
              ) : (
                <div className="space-y-1">
                  {categoryTree.map((cat) => renderCategoryItem(cat))}
                </div>
              )}
            </div>
          )}

          {/* 标签 Tab 内容 */}
          {activeTab === "tag" && renderTagSection()}

          {/* 分类表单弹窗 */}
          {activeTab === "category" && showNewForm && renderCategoryForm()}

          {/* 标签创建弹窗 */}
          {activeTab === "tag" && showCreateModal && renderTagCreateModal()}

          {/* 导入分类弹窗 */}
          {showImportDialog && (
            <ImportCategoryDialog
              categories={categories}
              onClose={() => setShowImportDialog(false)}
              onImported={loadCategories}
            />
          )}
        </div>
      </div>
    </div>
  );
}
