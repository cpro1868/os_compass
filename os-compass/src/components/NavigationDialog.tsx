interface NavigationItem {
  icon: string;
  label: string;
  description?: string;
  href?: string;
  onClick?: () => void;
  badge?: string;
}

interface NavigationCategory {
  title: string;
  items: NavigationItem[];
}

const navigationData: NavigationCategory[] = [
  {
    title: "项目管理",
    items: [
      { icon: "fa-solid fa-table-columns", label: "看板视图", description: "项目看板", onClick: () => window.dispatchEvent(new CustomEvent("navigate", { detail: { view: "kanban" } })) },
      { icon: "fa-solid fa-list", label: "列表视图", description: "高级筛选", onClick: () => window.dispatchEvent(new CustomEvent("navigate", { detail: { view: "list" } })) },
      { icon: "fa-solid fa-layer-group", label: "组织管理", description: "分类/标签", onClick: () => window.dispatchEvent(new CustomEvent("navigate", { detail: { view: "organization" } })) },
    ],
  },
  {
    title: "数据导入",
    items: [
      { icon: "fa-solid fa-plus-circle", label: "导入项目", description: "URL 自动解析", onClick: () => window.dispatchEvent(new CustomEvent("openImport")) },
      { icon: "fa-solid fa-pen", label: "手动录入", description: "私有/内网项目", onClick: () => window.dispatchEvent(new CustomEvent("openManualAdd")) },
    ],
  },
  {
    title: "插件功能",
    items: [
      { icon: "fa-solid fa-satellite-dish", label: "情报雷达", description: "RSS 聚合采集", onClick: () => window.dispatchEvent(new CustomEvent("navigate", { detail: { view: "radar" } })) },
      { icon: "fa-solid fa-magnifying-glass", label: "意图搜索", description: "全文检索", onClick: () => window.dispatchEvent(new CustomEvent("navigate", { detail: { view: "search" } })) },
    ],
  },
  {
    title: "系统",
    items: [
      { icon: "fa-solid fa-gear", label: "设置", description: "通用/下载/LLM", onClick: () => window.dispatchEvent(new CustomEvent("openSettings")) },
      { icon: "fa-solid fa-folder-open", label: "仓库管理", description: "多仓库/导入", onClick: () => window.dispatchEvent(new CustomEvent("openVault")) },
      { icon: "fa-solid fa-wand-magic-sparkles", label: "初始化向导", description: "创建/选择仓库", onClick: () => window.dispatchEvent(new CustomEvent("openInitWizard")) },
    ],
  },
];

interface NavigationDialogProps {
  open: boolean;
  onClose: () => void;
}

export function NavigationDialog({ open, onClose }: NavigationDialogProps) {
  if (!open) return null;

  const handleNavigate = (item: NavigationItem) => {
    if (item.onClick) {
      item.onClick();
    }
    onClose();
  };

  return (
    <div className="fixed inset-0 z-[100]">
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />
      <div className="absolute left-20 top-0 bottom-0 w-[720px] bg-white dark:bg-gray-800 shadow-2xl animate-slide-in">
        <div className="flex flex-col h-full">
          <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 bg-blue-600 rounded-xl flex items-center justify-center">
                <i className="fa-solid fa-compass text-white text-lg"></i>
              </div>
              <div>
                <h2 className="font-bold text-lg text-gray-900 dark:text-gray-100">OS-Compass</h2>
                <p className="text-sm text-gray-500 dark:text-gray-400">开源罗盘 · AI 开源资产智能管家</p>
              </div>
            </div>
            <button
              onClick={onClose}
              className="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition"
            >
              <i className="fa-solid fa-xmark text-lg"></i>
            </button>
          </div>

          <div className="flex-1 overflow-y-auto p-6">
            {navigationData.map((category, idx) => (
              <div key={idx} className="mb-8">
                <h3 className="text-sm font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-4">
                  {category.title}
                </h3>
                <div className="grid grid-cols-3 gap-5">
                  {category.items.map((item, itemIdx) => (
                    <button
                      key={itemIdx}
                      onClick={() => handleNavigate(item)}
                      className="flex items-start gap-4 p-5 rounded-xl hover:bg-gray-100 dark:hover:bg-gray-700 transition text-left group"
                    >
                      <div className={`w-12 h-12 rounded-xl flex items-center justify-center flex-shrink-0 ${
                        category.title === "项目管理" ? "bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400" :
                        category.title === "数据导入" ? "bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400" :
                        category.title === "插件功能" ? "bg-purple-100 dark:bg-purple-900/30 text-purple-600 dark:text-purple-400" :
                        "bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400"
                      }`}>
                        <i className={`${item.icon} text-xl`}></i>
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="font-medium text-gray-900 dark:text-gray-100 group-hover:text-blue-600 dark:group-hover:text-blue-400">
                          {item.label}
                          {item.badge && (
                            <span className="ml-2 px-2 py-0.5 text-xs bg-red-500 text-white rounded-full">
                              {item.badge}
                            </span>
                          )}
                        </div>
                        {item.description && (
                          <div className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                            {item.description}
                          </div>
                        )}
                      </div>
                    </button>
                  ))}
                </div>
              </div>
            ))}
          </div>

          <div className="px-6 py-4 border-t border-gray-200 dark:border-gray-700">
            <p className="text-xs text-gray-400 dark:text-gray-500 text-center">
              导航页 · 快捷访问所有功能
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
