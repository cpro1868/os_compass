import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { tagApi, type Tag } from "../api/tag";

interface TagManagerProps {
  onBack: () => void;
}

type TagSource = "ai" | "user" | "preset";

export function TagManager({ onBack }: TagManagerProps) {
  const { t } = useTranslation();
  const [tags, setTags] = useState<Tag[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [filterSource, setFilterSource] = useState<TagSource | "">("");
  const [loading, setLoading] = useState(true);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newTags, setNewTags] = useState("");
  const [newTagSource, setNewTagSource] = useState<TagSource>("user");
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    loadTags();
  }, []);

  const loadTags = async () => {
    try {
      const allTags = await tagApi.getAll();
      setTags(allTags);
    } catch (e) {
      console.error("Failed to load tags:", e);
    } finally {
      setLoading(false);
    }
  };

  const handleCreateTags = async () => {
    const tagNames = newTags.split("\n").map((t) => t.trim()).filter(Boolean);
    if (tagNames.length === 0) return;

    setCreating(true);
    try {
      for (const name of tagNames) {
        await tagApi.create({
          name,
          source: newTagSource,
        });
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
    ai: { bg: "bg-purple-100", text: "text-purple-700", icon: "fa-solid fa-robot" },
    user: { bg: "bg-pink-100", text: "text-pink-700", icon: "fa-solid fa-user" },
    preset: { bg: "bg-gray-100", text: "text-gray-700", icon: "fa-solid fa-star" },
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

  return (
    <div className="flex flex-col h-full">
      <header className="bg-white border-b border-gray-200 px-6 py-4">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-4">
            <button
              onClick={onBack}
              className="text-gray-500 hover:text-gray-700"
            >
              <i className="fa-solid fa-arrow-left mr-2"></i>{t("common.close")}
            </button>
            <h1 className="text-xl font-bold">{t("tag.title")}</h1>
          </div>
          <button
            onClick={() => setShowCreateModal(true)}
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
          >
            <i className="fa-solid fa-plus mr-2"></i>{t("tag.create")}
          </button>
        </div>

        <div className="flex gap-4">
          <div className="flex-1 relative">
            <i className="fa-solid fa-search absolute left-3 top-1/2 -translate-y-1/2 text-gray-400"></i>
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={`${t("tag.name")}...`}
              className="w-full pl-10 pr-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
          <select
            value={filterSource}
            onChange={(e) => setFilterSource(e.target.value as TagSource | "")}
            className="px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">{t("kanban.all")} {t("tag.source")}</option>
            <option value="ai">{t("tag.aiSource")}</option>
            <option value="user">{t("tag.userSource")}</option>
            <option value="preset">{t("tag.presetSource")}</option>
          </select>
        </div>
      </header>

      <div className="flex-1 overflow-auto p-6">
        {loading ? (
          <div className="text-center py-12 text-gray-500">
            <i className="fa-solid fa-spinner fa-spin text-2xl"></i>
            <p className="mt-2">{t("common.loading")}</p>
          </div>
        ) : (
          <div className="space-y-3">
            {(!filterSource || filterSource === "ai") && (
              <div className="bg-white rounded-xl border border-gray-200 p-4">
                <h3 className="font-medium text-gray-700 mb-3">
                  <i className={`${sourceColors.ai.icon} text-blue-500 mr-2`}></i>
                  {sourceNames.ai}
                  <span className="text-sm font-normal text-gray-400 ml-2">
                    （{sourceDescs.ai}）
                  </span>
                </h3>
                <div className="flex flex-wrap gap-2">
                  {aiTags.length === 0 ? (
                    <span className="text-gray-400 text-sm">暂无 AI 提取标签</span>
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
              <div className="bg-white rounded-xl border border-gray-200 p-4">
                <h3 className="font-medium text-gray-700 mb-3">
                  <i className={`${sourceColors.user.icon} text-green-500 mr-2`}></i>
                  {sourceNames.user}
                  <span className="text-sm font-normal text-gray-400 ml-2">
                    （{sourceDescs.user}）
                  </span>
                </h3>
                <div className="flex flex-wrap gap-2">
                  {userTags.length === 0 ? (
                    <span className="text-gray-400 text-sm">暂无用户添加标签</span>
                  ) : (
                    userTags.map((tag) => (
                      <span
                        key={tag.id}
                        className={`px-3 py-1.5 ${sourceColors.user.bg} ${sourceColors.user.text} rounded-full text-sm flex items-center gap-2`}
                      >
                        {tag.name}
                        <button
                          onClick={() => handleDeleteTag(tag.id)}
                          className="hover:text-pink-900"
                        >
                          <i className="fa-solid fa-times text-xs"></i>
                        </button>
                      </span>
                    ))
                  )}
                </div>
              </div>
            )}

            {(!filterSource || filterSource === "preset") && (
              <div className="bg-white rounded-xl border border-gray-200 p-4">
                <h3 className="font-medium text-gray-700 mb-3">
                  <i className={`${sourceColors.preset.icon} text-amber-500 mr-2`}></i>
                  {sourceNames.preset}
                  <span className="text-sm font-normal text-gray-400 ml-2">
                    （{sourceDescs.preset}）
                  </span>
                </h3>
                <div className="flex flex-wrap gap-2">
                  {presetTags.length === 0 ? (
                    <span className="text-gray-400 text-sm">暂无预设标签</span>
                  ) : (
                    presetTags.map((tag) => (
                      <span
                        key={tag.id}
                        className={`px-3 py-1.5 ${sourceColors.preset.bg} ${sourceColors.preset.text} rounded-full text-sm flex items-center gap-2`}
                      >
                        {tag.name}
                      </span>
                    ))
                  )}
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {showCreateModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl shadow-xl w-full max-w-md p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold">新建标签</h3>
              <button
                onClick={() => setShowCreateModal(false)}
                className="text-gray-400 hover:text-gray-600"
              >
                <i className="fa-solid fa-xmark text-xl"></i>
              </button>
            </div>

            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2 text-gray-700">
                  标签名称
                </label>
                <textarea
                  value={newTags}
                  onChange={(e) => setNewTags(e.target.value)}
                  placeholder="输入标签名称（每行一个，可批量添加）"
                  rows={5}
                  className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                  autoFocus
                />
                <p className="text-xs text-gray-500 mt-1">
                  每行一个标签，支持批量添加多个
                </p>
              </div>

              <div>
                <label className="block text-sm font-medium mb-2 text-gray-700">
                  标签类型
                </label>
                <select
                  value={newTagSource}
                  onChange={(e) => setNewTagSource(e.target.value as TagSource)}
                  className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                >
                  <option value="user">用户添加</option>
                  <option value="preset">预设</option>
                </select>
              </div>

              <div className="flex justify-end gap-3 pt-4 border-t">
                <button
                  onClick={() => setShowCreateModal(false)}
                  className="px-4 py-2 border rounded-lg hover:bg-gray-50"
                >
                  取消
                </button>
                <button
                  onClick={handleCreateTags}
                  disabled={!newTags.trim() || creating}
                  className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
                >
                  {creating ? "创建中..." : "创建"}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
