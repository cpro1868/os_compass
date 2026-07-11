import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { categoryApi, type Category } from "../api/category";

interface MoveCategoryDialogProps {
  open: boolean;
  selectedCount: number;
  onClose: () => void;
  onConfirm: (categoryId: number | null) => void;
}

export function MoveCategoryDialog({
  open,
  selectedCount,
  onClose,
  onConfirm,
}: MoveCategoryDialogProps) {
  const { t } = useTranslation();
  const [categories, setCategories] = useState<Category[]>([]);
  const [selectedCategoryId, setSelectedCategoryId] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (open) {
      categoryApi.getAll()
        .then(setCategories)
        .catch(() => {});
    }
  }, [open]);

  const handleConfirm = async () => {
    setLoading(true);
    try {
      onConfirm(selectedCategoryId);
      onClose();
    } finally {
      setLoading(false);
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-xl shadow-xl w-full max-w-md">
        <div className="p-6 border-b border-gray-200">
          <div className="flex items-center justify-between">
            <h2 className="text-xl font-semibold">{t("moveCategory.title")}</h2>
            <button
              onClick={onClose}
              className="text-gray-500 hover:text-gray-700"
            >
              <i className="fa-solid fa-xmark"></i>
            </button>
          </div>
          <p className="text-sm text-gray-500 mt-1">
            {t("moveCategory.selected", { count: selectedCount })}
          </p>
        </div>

        <div className="p-6">
          <label className="block text-sm font-medium mb-2">{t("moveCategory.category")}</label>
          <select
            value={selectedCategoryId ?? ""}
            onChange={(e) => setSelectedCategoryId(e.target.value ? Number(e.target.value) : null)}
            className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500"
          >
            <option value="">{t("moveCategory.uncategorized")}</option>
            {categories.map((cat) => (
              <option key={cat.id} value={cat.id}>
                {cat.name}
              </option>
            ))}
          </select>
          <p className="text-xs text-gray-500 mt-2">
            {t("moveCategory.tip")}
          </p>
        </div>

        <div className="p-6 border-t border-gray-200 flex gap-3">
          <button
            onClick={onClose}
            className="flex-1 px-4 py-2 border rounded-lg hover:bg-gray-50"
          >
            {t("moveCategory.cancel")}
          </button>
          <button
            onClick={handleConfirm}
            disabled={loading}
            className="flex-1 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
          >
            {loading ? t("moveCategory.moving") : t("moveCategory.confirm")}
          </button>
        </div>
      </div>
    </div>
  );
}
