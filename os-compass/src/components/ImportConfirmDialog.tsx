import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { categoryApi, type Category } from "../api/category";
import { tagApi } from "../api/tag";
import { MarkdownEditor } from "./MarkdownEditor";

interface ImportConfirmDialogProps {
  open: boolean;
  url: string;
  projectName?: string;
  markdownContent?: string;
  isUnknownPlatform?: boolean;
  onClose: () => void;
  onConfirm: (data: {
    name: string;
    category_id?: number;
    tags: string[];
    readme_content: string;
  }) => void;
}

export function ImportConfirmDialog({
  open,
  url,
  projectName = "",
  markdownContent = "",
  isUnknownPlatform = false,
  onClose,
  onConfirm,
}: ImportConfirmDialogProps) {
  const { t } = useTranslation();
  const [name, setName] = useState(projectName);
  const [categoryId, setCategoryId] = useState<number | null>(null);
  const [categories, setCategories] = useState<Category[]>([]);
  const [tags, setTags] = useState<string[]>([]);
  const [newTag, setNewTag] = useState("");
  const [readmeContent, setReadmeContent] = useState(markdownContent);

  useEffect(() => {
    if (open) {
      setName(projectName);
      setReadmeContent(markdownContent);
      categoryApi.getAll().then(setCategories).catch(() => {});
      tagApi.getAll().then((tags) => setTags(tags.map((t) => t.name))).catch(() => {});
    }
  }, [open, projectName, markdownContent]);

  const extractNameFromUrl = () => {
    try {
      const pathParts = new URL(url).pathname.split("/").filter(Boolean);
      if (pathParts.length >= 2) {
        const extracted = pathParts[pathParts.length - 1]
          .replace(/[-_]/g, " ")
          .replace(/[【】\[\]（）\(\)]/g, "")
          .trim();
        setName(extracted);
      }
    } catch {
      console.error("Failed to extract name from URL");
    }
  };

  const handleAddTag = () => {
    if (newTag.trim() && !tags.includes(newTag.trim())) {
      setTags([...tags, newTag.trim()]);
      setNewTag("");
    }
  };

  const handleRemoveTag = (tagToRemove: string) => {
    setTags(tags.filter((t) => t !== tagToRemove));
  };

  const handleConfirm = () => {
    if (!name.trim()) return;
    onConfirm({
      name: name.trim(),
      category_id: categoryId ?? undefined,
      tags,
      readme_content: readmeContent,
    });
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-6">
      <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-3xl max-h-[90vh] overflow-hidden flex flex-col">
        <div className="p-6 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <h2 className="text-xl font-semibold">{t("importConfirm.title")}</h2>
            <button
              onClick={onClose}
              className="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
            >
              <i className="fa-solid fa-xmark"></i>
            </button>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto p-6 space-y-5">
          {isUnknownPlatform && (
            <div className="bg-amber-50 dark:bg-amber-950 border border-amber-200 dark:border-amber-800 rounded-lg p-4">
              <div className="flex items-start gap-3">
                <i className="fa-solid fa-exclamation-triangle text-amber-500 dark:text-amber-400 text-xl mt-0.5"></i>
                <div>
                  <h3 className="font-medium text-amber-800 dark:text-amber-300">{t("importConfirm.unknownPlatformTitle")}</h3>
                  <p className="text-sm text-amber-700 dark:text-amber-300 mt-1">
                    {t("importConfirm.unknownPlatformDesc")}
                    <strong>{t("importConfirm.cannotScore")}</strong>。
                  </p>
                </div>
              </div>
            </div>
          )}

          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              {t("importConfirm.sourceUrl")}
            </label>
            <div className="px-4 py-3 bg-gray-50 dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 font-mono text-sm break-all">
              {url}
            </div>
          </div>

          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                {t("importConfirm.projectName")} <span className="text-red-500">*</span>
              </label>
              <button
                type="button"
                onClick={extractNameFromUrl}
                className="text-xs text-blue-600 dark:text-blue-400 hover:underline"
              >
                <i className="fa-solid fa-wand-magic-sparkles mr-1"></i>
                {t("importConfirm.extractFromUrl")}
              </button>
            </div>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
              placeholder={t("importConfirm.projectNamePlaceholder")}
            />
          </div>

          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="text-sm font-medium text-gray-700 dark:text-gray-300">{t("importConfirm.category")}</label>
              <button
                type="button"
                className="text-xs text-blue-600 dark:text-blue-400 hover:underline"
              >
                <i className="fa-solid fa-wand-magic-sparkles mr-1"></i>
                {t("importConfirm.autoCategory")}
              </button>
            </div>
            <select
              value={categoryId ?? ""}
              onChange={(e) => setCategoryId(e.target.value ? Number(e.target.value) : null)}
              className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
            >
              <option value="">{t("importConfirm.autoSelect")}</option>
              {categories.map((cat) => (
                <option key={cat.id} value={cat.id}>
                  {cat.name}
                </option>
              ))}
            </select>
          </div>

          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="text-sm font-medium text-gray-700 dark:text-gray-300">{t("importConfirm.tags")}</label>
              <button
                type="button"
                className="text-xs text-blue-600 dark:text-blue-400 hover:underline"
              >
                <i className="fa-solid fa-wand-magic-sparkles mr-1"></i>
                {t("importConfirm.autoExtractTags")}
              </button>
            </div>
            <div className="min-h-[42px] px-4 py-2 border rounded-lg bg-white dark:bg-gray-800 dark:border-gray-600">
              <div className="flex flex-wrap gap-2">
                {tags.map((tag) => (
                  <span
                    key={tag}
                    className="px-3 py-1.5 bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300 rounded-full text-sm flex items-center gap-1"
                  >
                    {tag}
                    <button
                      type="button"
                      onClick={() => handleRemoveTag(tag)}
                      className="hover:text-purple-900 dark:hover:text-purple-100"
                    >
                      <i className="fa-solid fa-times text-xs"></i>
                    </button>
                  </span>
                ))}
                <input
                  type="text"
                  value={newTag}
                  onChange={(e) => setNewTag(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      handleAddTag();
                    }
                  }}
                  placeholder={t("importConfirm.tagPlaceholder")}
                  className="mt-2 w-full text-sm border-0 focus:ring-0 p-0 dark:bg-gray-800 dark:text-gray-200"
                />
              </div>
            </div>
          </div>

          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                {t("importConfirm.readmeContent")}
              </label>
              <span className="text-xs text-amber-600 dark:text-amber-400">
                <i className="fa-solid fa-spider mr-1"></i>
                {t("importConfirm.convertedByCrawler")}
              </span>
            </div>
            <MarkdownEditor
              initialValue={readmeContent}
              onChange={setReadmeContent}
              minHeight="300px"
            />
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              <i className="fa-solid fa-info-circle mr-1"></i>
              {t("importConfirm.readmeEditTip")}
            </p>
          </div>

          {isUnknownPlatform && (
            <div className="bg-orange-50 dark:bg-orange-950 border border-orange-200 dark:border-orange-800 rounded-lg p-3">
              <p className="text-xs text-orange-700 dark:text-orange-300">
                <i className="fa-solid fa-info-circle mr-1"></i>
                <strong>{t("common.warning")}：</strong>{t("importConfirm.unknownPlatformNote")}
              </p>
            </div>
          )}
        </div>

        <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex gap-4">
          <button
            onClick={onClose}
            className="flex-1 px-4 py-3 border dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700"
          >
            {t("importConfirm.cancel")}
          </button>
          <button
            onClick={handleConfirm}
            disabled={!name.trim()}
            className="flex-1 px-4 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
          >
            {t("importConfirm.confirm")}
          </button>
        </div>
      </div>
    </div>
  );
}
