import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { categoryApi } from "../api/category";
import type { Category } from "../api/category";

interface ManualAddDialogProps {
  open: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

export function ManualAddDialog({ open, onClose, onSuccess }: ManualAddDialogProps) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const [description, setDescription] = useState("");
  const [categoryId, setCategoryId] = useState<number | null>(null);
  const [categories, setCategories] = useState<Category[]>([]);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open) {
      categoryApi.getAll().then(setCategories).catch(() => {});
    }
  }, [open]);

  const handleSubmit = async () => {
    if (!name.trim()) return;

    setSaving(true);
    try {
      await fetch("/api/projects", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: name.trim(),
          url: url.trim() || null,
          description: description.trim() || null,
          category_id: categoryId,
          source: "manual",
        }),
      });
      onSuccess();
      onClose();
      setName("");
      setUrl("");
      setDescription("");
      setCategoryId(null);
    } catch (e) {
      console.error("Failed to save project:", e);
    } finally {
      setSaving(false);
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-xl shadow-xl w-full max-w-xl">
        <div className="p-6 border-b border-gray-200">
          <div className="flex items-center justify-between">
            <h2 className="text-xl font-semibold">{t("manualAdd.title")}</h2>
            <button
              onClick={onClose}
              className="text-gray-500 hover:text-gray-700"
            >
              <i className="fa-solid fa-xmark"></i>
            </button>
          </div>
          <p className="text-sm text-gray-500 mt-1">
            {t("manualAdd.subtitle")}
          </p>
        </div>

        <div className="p-6 space-y-4">
          <div>
            <label className="block text-sm font-medium mb-2">
              {t("manualAdd.name")} <span className="text-red-500">*</span>
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={t("manualAdd.namePlaceholder")}
              className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-2">
              {t("manualAdd.url")}
            </label>
            <input
              type="text"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder={t("manualAdd.urlPlaceholder")}
              className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-2">
              {t("manualAdd.desc")}
            </label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={t("manualAdd.descPlaceholder")}
              rows={3}
              className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-2">
              {t("manualAdd.category")}
            </label>
            <select
              value={categoryId ?? ""}
              onChange={(e) => setCategoryId(e.target.value ? Number(e.target.value) : null)}
              className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500"
            >
              <option value="">{t("manualAdd.uncategorized")}</option>
              {categories.map((cat) => (
                <option key={cat.id} value={cat.id}>
                  {cat.name}
                </option>
              ))}
            </select>
          </div>

          <div className="bg-blue-50 border border-blue-200 rounded-lg p-3">
            <p className="text-sm text-blue-700">
              <i className="fa-solid fa-info-circle mr-1"></i>
              {t("manualAdd.infoTip")}
            </p>
          </div>
        </div>

        <div className="p-6 border-t border-gray-200 flex gap-3">
          <button
            onClick={onClose}
            className="flex-1 px-4 py-2 border rounded-lg hover:bg-gray-50"
          >
            {t("manualAdd.cancel")}
          </button>
          <button
            onClick={handleSubmit}
            disabled={!name.trim() || saving}
            className="flex-1 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
          >
            {saving ? t("manualAdd.saving") : t("manualAdd.save")}
          </button>
        </div>
      </div>
    </div>
  );
}
