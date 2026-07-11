import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import type { Category } from "../types";
import * as api from "../api";
import { EmptyState } from "./EmptyState";

interface CategoryNode extends Category {
  children?: CategoryNode[];
  projectCount?: number;
}

interface CategoryManagerProps {
  onBack: () => void;
}

export function CategoryManager({ onBack }: CategoryManagerProps) {
  const { t } = useTranslation();
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);
  const [expandedIds, setExpandedIds] = useState<Set<number>>(new Set([1]));
  const [editingCategory, setEditingCategory] = useState<Category | null>(null);
  const [showNewForm, setShowNewForm] = useState(false);
  const [lastSelectedId, setLastSelectedId] = useState<number | null>(null);
  const [draggingId, setDraggingId] = useState<number | null>(null);
  const [dragOverId, setDragOverId] = useState<number | null>(null);
  const [dragOverPos, setDragOverPos] = useState<"before" | "inside" | "after" | null>(null);
  const [formData, setFormData] = useState({
    name: "",
    parent_id: null as number | null,
    sort_order: 0,
    explain: "",
  });

  useEffect(() => {
    loadCategories();
  }, []);

  const loadCategories = async () => {
    setLoading(true);
    try {
      const cats = await api.getCategories();
      setCategories(cats);
      setExpandedIds(new Set([1]));
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
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

  const handleSave = async () => {
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

  const handleDelete = async (id: number) => {
    if (confirm("确定要删除此分类吗？")) {
      try {
        await api.deleteCategory(id);
        loadCategories();
      } catch (e) {
        console.error(e);
      }
    }
  };

  const openEdit = (cat: Category) => {
    setEditingCategory(cat);
    setFormData({
      name: cat.name,
      parent_id: cat.parent_id,
      sort_order: cat.sort_order || 0,
      explain: cat.explain || "",
    });
  };

  const openNew = (parentId: number | null = null) => {
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
            lastSelectedId === cat.id ? "bg-blue-100 border border-blue-300" : "bg-gray-50 hover:bg-gray-100"
          } ${isDragOver && dropIndicator === "inside" ? "ring-2 ring-indigo-400 bg-indigo-50" : ""} ${
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
              className="w-6 h-6 flex items-center justify-center text-gray-500 hover:text-blue-600 hover:bg-blue-50 rounded"
              title={hasChildren ? (isExpanded ? "收起" : "展开") : ""}
            >
              {hasChildren ? (
                <i className={`fa-solid ${isExpanded ? "fa-chevron-down" : "fa-chevron-right"}`}></i>
              ) : (
                <i className="fa-solid fa-circle text-xs text-gray-300"></i>
              )}
            </button>
            <i className={`fa-solid fa-folder ${hasChildren ? "text-yellow-500" : "text-yellow-300"}`}></i>
            <span className={`font-medium ${isSystem ? "text-gray-600" : "text-gray-800"}`}>{cat.name}</span>
            {isSystem && <span className="text-xs text-gray-400 bg-gray-200 px-2 py-0.5 rounded">{t("category.system")}</span>}
            {hasChildren && (
              <span className="text-xs text-gray-400">({cat.children!.length})</span>
            )}
          </div>
          <div className="flex items-center gap-1">
            {!isSystem && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  openNew(cat.id);
                }}
                className="p-1.5 text-gray-400 hover:text-green-600 hover:bg-green-50 rounded"
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
                    openEdit(cat);
                  }}
                  className="p-1.5 text-gray-400 hover:text-blue-600 hover:bg-blue-50 rounded"
                  title={t("category.edit")}
                >
                  <i className="fa-solid fa-pen-to-square"></i>
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDelete(cat.id);
                  }}
                  className="p-1.5 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded"
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

  return (
    <div className="flex flex-col h-full">
      <header className="bg-white border-b border-gray-200 px-6 py-4 flex items-center justify-between">
        <div className="flex items-center gap-4">
          <button
            onClick={onBack}
            className="text-gray-500 hover:text-gray-700"
          >
            <i className="fa-solid fa-arrow-left mr-2"></i>{t("common.close")}
          </button>
          <h1 className="text-xl font-bold">{t("category.title")}</h1>
        </div>
        <button
          onClick={() => openNew(null)}
          className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
        >
          <i className="fa-solid fa-plus mr-2"></i>{t("category.create")}
        </button>
      </header>

      <div className="flex-1 overflow-auto p-6">
        <div className="bg-white rounded-xl border border-gray-200 p-6 max-w-4xl">
          {loading ? (
            <div className="text-center py-8 text-gray-500">
              <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mb-2"></div>
              <p className="text-sm">{t("category.loading")}</p>
            </div>
          ) : categoryTree.length === 0 ? (
            <EmptyState
              icon="fa-folder-tree"
              title={t("category.empty")}
              description={t("category.emptyDesc")}
              actionLabel={t("category.createFirst")}
              onAction={() => openNew(null)}
            />
          ) : (
            categoryTree.map((cat) => renderCategory(cat))
          )}

          {showNewForm && (
            <div className="bg-gray-50 rounded-xl border border-gray-200 p-6 mt-6">
              <h3 className="font-bold mb-4 text-lg">
                {editingCategory ? `${t("category.form.editTitle")}：${editingCategory.name}` : t("category.form.newTitle")}
              </h3>
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium mb-1 text-gray-700">{t("category.form.name")}</label>
                  <input
                    type="text"
                    value={formData.name}
                    onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                    className="w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    placeholder={t("category.form.namePlaceholder")}
                    autoFocus
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium mb-1 text-gray-700">{t("category.form.parent")}</label>
                  <select
                    value={formData.parent_id || ""}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        parent_id: e.target.value ? Number(e.target.value) : null,
                      })
                    }
                    className="w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                  >
                    <option value="">{t("category.form.topLevel")}</option>
                    {renderParentOptions(categories.filter((c) => c.parent_id === null))}
                  </select>
                  <p className="text-xs text-gray-500 mt-1">
                    {formData.parent_id
                      ? t("category.willCreateUnder", { name: categories.find((c) => c.id === formData.parent_id)?.name })
                      : t("category.willCreateTop")}
                  </p>
                </div>
                <div>
                  <label className="block text-sm font-medium mb-1 text-gray-700">{t("category.form.desc")}</label>
                  <textarea
                    value={formData.explain}
                    onChange={(e) => setFormData({ ...formData, explain: e.target.value })}
                    rows={3}
                    placeholder={t("category.form.descPlaceholder")}
                    className="w-full px-3 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                  />
                  <p className="text-xs text-gray-500 mt-1">{t("category.form.descTip")}</p>
                </div>
                <div className="flex justify-end gap-3 pt-4 border-t border-gray-200">
                  <button
                    onClick={() => {
                      setEditingCategory(null);
                      setShowNewForm(false);
                    }}
                    className="px-4 py-2 border rounded-lg hover:bg-gray-100"
                  >
                    {t("category.form.cancel")}
                  </button>
                  <button
                    onClick={handleSave}
                    disabled={!formData.name.trim()}
                    className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
                  >
                    保存
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
