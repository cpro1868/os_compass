import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import type { Category } from "../types";
import * as api from "../api";
import { tagApi, type Tag } from "../api/tag";

interface CategoryNode extends Category {
  children?: CategoryNode[];
  projectCount?: number;
}

type TabType = "category" | "tag";
type TagSource = "ai" | "user" | "preset";

export function OrganizationManager() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<TabType>("category");

  // Category state
  const [categories, setCategories] = useState<Category[]>([]);
  const [loadingCategories, setLoadingCategories] = useState(true);
  const [expandedIds, setExpandedIds] = useState<Set<number>>(new Set([1]));
  const [editingCategory, setEditingCategory] = useState<Category | null>(null);
  const [showNewForm, setShowNewForm] = useState(false);
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

  // Load categories
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

  // Load tags
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

  useEffect(() => {
    loadCategories();
    loadTags();
  }, []);

  const buildTree = (cats: Category[], parentId: number | null = null): CategoryNode[] => {
    return cats
      .filter((c) => c.parent_id === parentId)
      .sort((a, b) => (a.sort_order || 0) - (b.sort_order || 0))
      .map((c) => ({
        ...c,
        children: buildTree(cats, c.id),
      }));
  };

  // Category handlers
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

  // Tag handlers
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

  const categoryTree = buildTree(categories);

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

  const renderCategoryItem = (cat: CategoryNode, level: number = 0) => (
    <div key={cat.id}>
      <div
        className={`flex items-center gap-2 px-3 py-2 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg group ${
          level > 0 ? "ml-6" : ""
        }`}
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
        {cat.children && cat.children.length > 0 && (
          <span className="text-xs text-gray-400 dark:text-gray-500">({cat.children.length})</span>
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
            </>
          )}
          <button
            onClick={() => openNewCategory(cat.id)}
            className="p-1 text-gray-400 hover:text-green-500"
          >
            <i className="fa-solid fa-plus text-xs"></i>
          </button>
        </div>
      </div>
      {expandedIds.has(cat.id) && cat.children && (
        <div className="mt-1 space-y-1">
          {cat.children.map((child) => renderCategoryItem(child, level + 1))}
        </div>
      )}
    </div>
  );

  return (
    <div className="flex flex-col h-full">
      <header className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
        <div className="flex items-center justify-between mb-4">
          <h1 className="text-xl font-bold dark:text-gray-100">{t("sidebar.organization") || "项目管理"}</h1>
        </div>

        {/* Tabs */}
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
        {/* Category Tab */}
        {activeTab === "category" && (
          <div className="space-y-4">
            <div className="flex justify-end">
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
            ) : (
              <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4 space-y-1">
                {categoryTree.map((cat) => renderCategoryItem(cat))}
              </div>
            )}
          </div>
        )}

        {/* Tag Tab */}
        {activeTab === "tag" && (
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
                      <span className="text-sm font-normal text-gray-400 ml-2">（{sourceDescs.user}）</span>
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
                      <span className="text-sm font-normal text-gray-400 ml-2">（{sourceDescs.preset}）</span>
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
        )}
      </div>

      {/* Category Form Modal */}
      {showNewForm && (
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
                  {t("category.name") || "分类名称"}
                </label>
                <input
                  type="text"
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                  className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                  autoFocus
                />
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
      )}

      {/* Tag Create Modal */}
      {showCreateModal && (
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
      )}
    </div>
  );
}
