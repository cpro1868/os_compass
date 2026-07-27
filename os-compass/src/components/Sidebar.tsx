import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { getCurrentVault } from "../api";
import { getRadarUnreadCount } from "../api/radar";

interface SidebarProps {
  onSettings: () => void;
  onOpenVault: () => void;
  onOpenCategory: () => void;
  onOpenNavigation: () => void;
  onOpenInitWizard: () => void;
  currentView: string;
  onViewChange: (view: "home" | "kanban" | "list" | "archive" | "category" | "tag" | "radar" | "search" | "organization") => void;
}

export function Sidebar({ onSettings, onOpenVault, onOpenCategory, onOpenNavigation, onOpenInitWizard, currentView, onViewChange }: SidebarProps) {
  const { t } = useTranslation();
  const [vaultName, setVaultName] = useState<string>("");
  const [unreadCount, setUnreadCount] = useState<number>(0);

  useEffect(() => {
    getCurrentVault().then((v) => {
      if (v) setVaultName(v.name);
    }).catch(() => {});
    const handler = () => {
      getCurrentVault().then((v) => {
        if (v) setVaultName(v.name);
      }).catch(() => {});
    };
    window.addEventListener("vault-changed", handler);
    return () => window.removeEventListener("vault-changed", handler);
  }, []);

  useEffect(() => {
    getRadarUnreadCount().then(setUnreadCount).catch(() => {});
  }, []);

  return (
    <aside className="w-16 bg-gray-900 dark:bg-gray-950 flex flex-col items-center py-4 gap-2 flex-shrink-0">
      {/* Logo */}
      <button
        onClick={onOpenNavigation}
        className="w-10 h-10 bg-gradient-to-br from-blue-500 to-indigo-600 rounded-xl flex items-center justify-center mb-2 hover:scale-105 transition-transform"
        title="导航"
      >
        <i className="fa-solid fa-compass text-white text-lg"></i>
      </button>

      {/* 导航 */}
      <button
        onClick={onOpenNavigation}
        className="w-12 h-12 rounded-xl hover:bg-gray-800 flex flex-col items-center justify-center text-gray-400 hover:text-white transition"
        title={t("sidebar.navigation") || "导航"}
      >
        <i className="fa-solid fa-compass text-lg mb-0.5"></i>
        <span className="text-xs">{t("sidebar.navigation") || "导航"}</span>
      </button>

      {/* 首页 */}
      <button
        onClick={() => onViewChange("home")}
        className={`w-12 h-10 rounded-lg flex items-center justify-center transition ${
          currentView === "home" ? "bg-blue-600 text-white" : "text-gray-400 hover:bg-gray-800 hover:text-white"
        }`}
        title={t("home.title") || "首页"}
      >
        <i className="fa-solid fa-house text-lg"></i>
      </button>

      {/* 视图切换 */}
      <div className="flex flex-col gap-1 mt-4 border-t border-gray-700 pt-4">
        <button
          onClick={() => onViewChange("kanban")}
          className={`w-12 h-10 rounded-lg flex items-center justify-center transition ${
            currentView === "kanban" ? "bg-gray-700 text-white" : "text-gray-400 hover:bg-gray-800 hover:text-white"
          }`}
          title={t("nav.kanban")}
        >
          <i className="fa-solid fa-table-columns"></i>
        </button>
        <button
          onClick={() => onViewChange("list")}
          className={`w-12 h-10 rounded-lg flex items-center justify-center transition ${
            currentView === "list" ? "bg-gray-700 text-white" : "text-gray-400 hover:bg-gray-800 hover:text-white"
          }`}
          title={t("nav.list")}
        >
          <i className="fa-solid fa-list"></i>
        </button>
        <div className="relative">
          <button
            onClick={() => onViewChange("radar")}
            className={`w-12 h-12 rounded-xl flex flex-col items-center justify-center transition ${
              currentView === "radar" ? "bg-blue-600 text-white" : "text-gray-400 hover:bg-gray-800 hover:text-white"
            }`}
            title={t("nav.radar")}
          >
            <i className="fa-solid fa-satellite-dish text-lg mb-0.5"></i>
            {unreadCount > 0 && (
              <span className="absolute -top-1 -right-1 bg-red-500 text-white text-xs rounded-full w-4 h-4 flex items-center justify-center">
                {unreadCount > 9 ? '9+' : unreadCount}
              </span>
            )}
          </button>
        </div>
        <button
          onClick={() => onViewChange("search")}
          className={`w-12 h-12 rounded-xl flex items-center justify-center transition ${
            currentView === "search" ? "bg-blue-600 text-white" : "text-gray-400 hover:bg-gray-800 hover:text-white"
          }`}
          title={t("nav.search")}
        >
          <i className="fa-solid fa-magnifying-glass text-lg"></i>
        </button>
      </div>

      {/* 分类/标签（合并视图） */}
      <button
        onClick={onOpenCategory}
        className={`w-12 h-10 rounded-lg flex items-center justify-center transition ${
          currentView === "category" || currentView === "tag" || currentView === "organization"
            ? "bg-gray-700 text-white"
            : "text-gray-400 hover:bg-gray-800 hover:text-white"
        }`}
        title={t("sidebar.organization") || "组织管理"}
      >
        <i className="fa-solid fa-layer-group"></i>
      </button>

      {/* 仓库信息（显示仓库名，点击打开仓库管理） */}
      <button
        onClick={onOpenVault}
        className="mt-2 px-1 py-2 bg-gray-800 rounded-lg text-center hover:bg-gray-700 transition w-12"
        title={`${vaultName || '仓库'} - 点击切换`}
      >
        <i className="fa-solid fa-database text-amber-500 text-sm"></i>
        <p className="text-[10px] text-gray-400 mt-1 truncate w-10">{vaultName || '仓库'}</p>
      </button>

      {/* 底部按钮 */}
      <div className="border-t border-gray-700 pt-4 mt-auto flex flex-col gap-1 w-full items-center">
        <button
          onClick={onSettings}
          className="w-12 h-10 rounded-lg flex items-center justify-center text-gray-400 hover:bg-gray-800 hover:text-white transition"
          title={t("sidebar.settings")}
        >
          <i className="fa-solid fa-gear"></i>
        </button>
        <button
          onClick={onOpenInitWizard}
          className="w-12 h-10 rounded-lg flex items-center justify-center text-gray-400 hover:bg-gray-800 hover:text-white transition"
          title="初始化向导"
        >
          <i className="fa-solid fa-wand-magic-sparkles"></i>
        </button>
      </div>
    </aside>
  );
}
