import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  listVaults,
  createVault,
  openVault,
  deleteVault,
  getCurrentVault,
  importVault,
} from "../api";

interface Vault {
  name: string;
  path: string;
  project_count: number;
}

interface VaultDialogProps {
  open: boolean;
  onClose: () => void;
}

export function VaultDialog({ open, onClose }: VaultDialogProps) {
  const { t } = useTranslation();
  const [vaults, setVaults] = useState<Vault[]>([]);
  const [currentVault, setCurrentVault] = useState<Vault | null>(null);
  const [newVaultName, setNewVaultName] = useState("");
  const [newVaultPath, setNewVaultPath] = useState("");
  const [createStep, setCreateStep] = useState({ step: 0, percent: 0, error: null as string | null });
  const [createButtonState, setCreateButtonState] = useState("idle");
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [switchModalOpen, setSwitchModalOpen] = useState(false);
  const [deleteModalOpen, setDeleteModalOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<Vault | null>(null);
  const [deleteMode, setDeleteMode] = useState<"index" | "permanent">("index");
  const [importModalOpen, setImportModalOpen] = useState(false);
  const [importVaultName, setImportVaultName] = useState("");
  const [importVaultPath, setImportVaultPath] = useState("");
  const [importButtonState, setImportButtonState] = useState<"idle" | "importing" | "error">("idle");
  const [importError, setImportError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      refreshData();
    }
  }, [open]);

  const refreshData = async () => {
    const [v, cv] = await Promise.all([listVaults(), getCurrentVault()]);
    setVaults(v);
    setCurrentVault(cv);
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-xl shadow-xl w-full max-w-4xl max-h-[80vh] overflow-hidden flex flex-col">
        {/* 头部 */}
        <div className="p-6 border-b border-gray-200 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <i className="fa-solid fa-folder-open text-amber-600 text-xl"></i>
            <h1 className="text-2xl font-bold">{t("vault.title")}</h1>
          </div>
          <button onClick={onClose} className="w-8 h-8 flex items-center justify-center text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg">
            <i className="fa-solid fa-xmark text-lg"></i>
          </button>
        </div>

        {/* 内容 */}
        <div className="flex-1 overflow-y-auto p-6">
          {/* 当前仓库信息卡片 */}
          {currentVault && (
            <div className="bg-gradient-to-r from-blue-600 to-indigo-600 rounded-xl p-6 mb-6 text-white">
              <div className="flex items-center justify-between">
                <div>
                  <div className="flex items-center gap-3 mb-2">
                    <i className="fa-solid fa-database text-2xl"></i>
                    <span className="text-lg font-medium">当前仓库</span>
                  </div>
                  <h2 className="text-2xl font-bold mb-1">{currentVault.name}</h2>
                  <p className="text-blue-100 text-sm truncate max-w-md">{currentVault.path}</p>
                </div>
                <div className="text-right">
                  <div className="text-4xl font-bold mb-1">{currentVault.project_count}</div>
                  <div className="text-blue-100 text-sm">{t("vault.list.projectCount")}</div>
                </div>
              </div>
              <div className="mt-4 flex gap-3">
                <button
                  onClick={() => setSwitchModalOpen(true)}
                  className="px-4 py-2 bg-white/20 hover:bg-white/30 rounded-lg text-sm flex items-center gap-2"
                >
                  <i className="fa-solid fa-right-left"></i>切换仓库
                </button>
                <button
                  onClick={() => invoke("open", { path: currentVault.path }).catch(() => {})}
                  className="px-4 py-2 bg-white/20 hover:bg-white/30 rounded-lg text-sm flex items-center gap-2"
                >
                  <i className="fa-solid fa-folder"></i>打开所在文件夹
                </button>
              </div>
            </div>
          )}

          {/* 仓库列表 */}
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-lg font-semibold">所有仓库</h2>
            <button
              onClick={refreshData}
              className="text-sm text-blue-600 hover:text-blue-700 flex items-center gap-1"
            >
              <i className="fa-solid fa-arrows-rotate"></i>刷新
            </button>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {vaults.map((vault) => (
              <div
                key={vault.path}
                className={`vault-card bg-white rounded-xl border p-5 ${
                  currentVault?.path === vault.path ? "border-2 border-blue-500" : "border-gray-200"
                }`}
              >
                <div className="flex items-start justify-between mb-4">
                  <div className={`w-12 h-12 rounded-xl flex items-center justify-center ${
                    currentVault?.path === vault.path ? "bg-blue-100" : "bg-purple-100"
                  }`}>
                    <i className={`fa-solid fa-database text-xl ${
                      currentVault?.path === vault.path ? "text-blue-600" : "text-purple-600"
                    }`}></i>
                  </div>
                  {currentVault?.path === vault.path && (
                    <span className="px-2 py-1 bg-blue-100 text-blue-700 text-xs font-medium rounded">{t("vault.list.current")}</span>
                  )}
                </div>
                <h3 className="font-semibold text-lg mb-1">{vault.name}</h3>
                <p className="text-gray-500 text-sm mb-3 break-all">{vault.path}</p>
                <div className="flex items-center gap-4 text-sm text-gray-500 mb-4">
                  <span><i className="fa-solid fa-layer-group mr-1"></i>{vault.project_count} {t("vault.list.projectCount")}</span>
                </div>
                <div className="flex gap-2">
                  {currentVault?.path !== vault.path && (
                    <button
                      onClick={async () => {
                        try {
                          console.log("[VaultDialog] Switching to vault:", vault.path);
                          await openVault(vault.path);
                          console.log("[VaultDialog] Switch successful");
                          window.dispatchEvent(new CustomEvent("vault-changed"));
                          await refreshData();
                        } catch (e) {
                          console.error("[VaultDialog] Switch failed:", e);
                          alert("切换仓库失败: " + String(e));
                        }
                      }}
                      className="flex-1 px-3 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm"
                    >
                      切换
                    </button>
                  )}
                  <button
                    onClick={() => {
                      setDeleteTarget(vault);
                      setDeleteModalOpen(true);
                    }}
                    className="px-3 py-2 text-red-600 hover:bg-red-50 rounded-lg text-sm"
                  >
                    <i className="fa-solid fa-trash"></i>
                  </button>
                </div>
              </div>
            ))}

            {/* 导入已有仓库卡片 */}
            <div
              onClick={() => setImportModalOpen(true)}
              className="vault-card bg-white rounded-xl border-2 border-dashed border-emerald-300 p-5 cursor-pointer hover:border-emerald-500 hover:bg-emerald-50/30"
            >
              <div className="flex flex-col items-center justify-center py-8">
                <div className="w-12 h-12 bg-emerald-50 rounded-xl flex items-center justify-center mb-4">
                  <i className="fa-solid fa-file-import text-emerald-500 text-xl"></i>
                </div>
                <p className="text-emerald-600 text-sm font-medium">{t("vault.import.title")}</p>
                <p className="text-gray-400 text-xs mt-1">选择含仓库文件的目录</p>
              </div>
            </div>

            {/* 新建仓库卡片 */}
            <div
              onClick={() => setCreateModalOpen(true)}
              className="vault-card bg-white rounded-xl border-2 border-dashed border-gray-300 p-5 cursor-pointer hover:border-blue-400 hover:bg-blue-50/30"
            >
              <div className="flex flex-col items-center justify-center py-8">
                <div className="w-12 h-12 bg-gray-100 rounded-xl flex items-center justify-center mb-4">
                  <i className="fa-solid fa-plus text-gray-400 text-xl"></i>
                </div>
                <p className="text-gray-500 text-sm">点击创建新仓库</p>
              </div>
            </div>
          </div>

          {/* 提示信息 */}
          <div className="mt-6 bg-amber-50 border border-amber-200 rounded-xl p-4 flex items-start gap-3">
            <i className="fa-solid fa-lightbulb text-amber-500 mt-0.5"></i>
            <div>
              <h4 className="font-medium text-amber-800 mb-1">使用提示</h4>
              <ul className="text-sm text-amber-700 space-y-1">
                <li>每个仓库是独立的数据存储空间，可用于分类管理不同场景的项目</li>
                <li>仓库数据以 SQLite 数据库文件形式存储，便于备份和迁移</li>
                <li>删除仓库不会影响其他仓库的数据</li>
              </ul>
            </div>
          </div>
        </div>
      </div>

      {/* 创建仓库弹窗 */}
      {createModalOpen && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-[60] p-4" onClick={() => createButtonState !== "creating" && setCreateModalOpen(false)}>
          <div className="bg-white rounded-xl shadow-xl w-full max-w-2xl max-h-[90vh] overflow-y-auto" onClick={(e) => e.stopPropagation()}>
            <div className="p-6 border-b border-gray-200">
              <div className="flex items-center justify-between">
                <h2 className="text-xl font-semibold">{t("vault.create.title")}</h2>
                {createButtonState !== "creating" && (
                  <button onClick={() => setCreateModalOpen(false)} className="text-gray-500 hover:text-gray-700">
                    <i className="fa-solid fa-xmark text-xl"></i>
                  </button>
                )}
              </div>
            </div>

            <div className="p-6 space-y-5">
              <div>
                <label className="block text-sm font-medium mb-2">{t("vault.create.name")} <span className="text-red-500">*</span></label>
                <input
                  type="text"
                  value={newVaultName}
                  onChange={(e) => setNewVaultName(e.target.value)}
                  placeholder={t("vault.create.namePlaceholder")}
                  className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                  disabled={createButtonState === "creating"}
                />
              </div>

              <div>
                <label className="block text-sm font-medium mb-2">{t("vault.create.path")} <span className="text-red-500">*</span></label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={newVaultPath}
                    onChange={(e) => setNewVaultPath(e.target.value)}
                    placeholder="选择存放目录"
                    className="flex-1 px-4 py-2 border rounded-lg bg-gray-50 focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    disabled={createButtonState === "creating"}
                  />
                  <button
                    type="button"
                    onClick={async () => {
                      try {
                        const selected = await openDialog({
                          directory: true,
                          multiple: false,
                          title: "选择仓库存放目录",
                        });
                        if (selected) {
                          setNewVaultPath(selected as string);
                        }
                      } catch (e) {
                        console.error("Failed to open directory dialog:", e);
                      }
                    }}
                    disabled={createButtonState === "creating"}
                    className="px-4 py-2 border rounded-lg hover:bg-gray-50 disabled:opacity-50"
                  >
                    <i className="fa-solid fa-folder-open"></i>
                  </button>
                </div>
              </div>

              <p className="text-xs text-gray-500">
                <i className="fa-solid fa-circle-info mr-1"></i>
                将在所选目录下创建 SQLite 数据库和加密密钥文件
              </p>

              {createStep.step > 0 && (
                <div className="border-t border-gray-200 pt-5">
                  <h4 className="font-medium mb-3">正在创建仓库...</h4>
                  <div className="space-y-3">
                    {[
                      { id: 1, text: "验证目录有效性", successText: "目录有效性验证通过" },
                      { id: 2, text: "创建数据库文件", successText: "数据库文件创建成功" },
                      { id: 3, text: "执行初始化脚本", successText: "初始化脚本执行完成" },
                      { id: 4, text: "验证数据库结构", successText: "数据库结构验证通过" },
                    ].map((step) => (
                      <div key={step.id} className="flex items-center gap-3">
                        <div className={`w-6 h-6 rounded-full flex items-center justify-center text-xs ${
                          createStep.step > step.id ? "bg-green-500 text-white" :
                          createStep.step === step.id ? "bg-blue-500 animate-pulse text-white" :
                          "bg-gray-200 text-gray-500"
                        }`}>
                          {createStep.step > step.id ? (
                            <i className="fa-solid fa-check"></i>
                          ) : (
                            <i className="fa-solid fa-circle"></i>
                          )}
                        </div>
                        <span className={`text-sm ${
                          createStep.step > step.id ? "text-green-600 font-medium" :
                          createStep.step === step.id ? "text-gray-500" :
                          "text-gray-400"
                        }`}>
                          {createStep.step > step.id ? step.successText : step.text}
                        </span>
                      </div>
                    ))}
                  </div>

                  <div className="mt-4">
                    <div className="flex justify-between text-sm mb-1">
                      <span className="text-gray-500">{createStep.step > 3 ? "完成" : `步骤 ${createStep.step}/4`}</span>
                      <span className="text-blue-600 font-medium">{createStep.percent}%</span>
                    </div>
                    <div className="w-full bg-gray-200 rounded-full h-2">
                      <div className="bg-blue-600 h-2 rounded-full transition-all duration-300" style={{ width: `${createStep.percent}%` }}></div>
                    </div>
                  </div>

                  {createStep.error && (
                    <div className="mt-4 bg-red-50 border border-red-200 rounded-lg p-3">
                      <div className="flex items-start gap-2">
                        <i className="fa-solid fa-circle-xmark text-red-500 mt-0.5"></i>
                        <div>
                          <p className="text-sm text-red-700 font-medium">创建失败</p>
                          <p className="text-sm text-red-600 mt-1">{createStep.error}</p>
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>

            <div className="p-6 border-t border-gray-200 flex gap-3">
              <button
                onClick={() => {
                  setCreateModalOpen(false);
                  setCreateStep({ step: 0, percent: 0, error: null });
                  setNewVaultName("");
                  setNewVaultPath("");
                  setCreateButtonState("idle");
                }}
                disabled={createButtonState === "creating"}
                className="flex-1 px-4 py-2 border rounded-lg hover:bg-gray-50 disabled:opacity-50"
              >
                {t("vault.delete.cancel")}
              </button>
              <button
                onClick={async () => {
                  if (!newVaultName.trim()) return;
                  if (!newVaultPath.trim()) {
                    alert("请选择存放目录");
                    return;
                  }

                  setCreateButtonState("creating");
                  setCreateStep({ step: 0, percent: 0, error: null });

                  try {
                    setCreateStep({ step: 1, percent: 10, error: null });
                    await new Promise((r) => setTimeout(r, 500));

                    await createVault(newVaultName.trim(), newVaultPath.trim());

                    setCreateStep({ step: 2, percent: 35, error: null });
                    await new Promise((r) => setTimeout(r, 600));

                    setCreateStep({ step: 3, percent: 60, error: null });
                    await new Promise((r) => setTimeout(r, 800));

                    setCreateStep({ step: 4, percent: 85, error: null });
                    await new Promise((r) => setTimeout(r, 400));

                    setCreateStep({ step: 5, percent: 100, error: null });
                    await new Promise((r) => setTimeout(r, 300));

                    await refreshData();
                    setCreateModalOpen(false);
                    setNewVaultName("");
                    setNewVaultPath("");
                    setCreateButtonState("idle");
                  } catch (err: any) {
                    setCreateStep((s) => ({ ...s, error: err.toString() }));
                    setCreateButtonState("error");
                  }
                }}
                disabled={createButtonState === "creating" || !newVaultName.trim() || !newVaultPath.trim()}
                className="flex-1 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
              >
                {createButtonState === "creating" ? (
                  <><i className="fa-solid fa-spinner fa-spin mr-2"></i>{t("vault.create.creating")}</>
                ) : t("vault.create.confirm")}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 切换仓库弹窗 */}
      {switchModalOpen && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-[60]" onClick={() => setSwitchModalOpen(false)}>
          <div className="bg-white rounded-xl shadow-xl w-full max-w-lg" onClick={(e) => e.stopPropagation()}>
            <div className="p-6 border-b border-gray-200">
              <div className="flex items-center justify-between">
                <h2 className="text-xl font-semibold">切换仓库</h2>
                <button onClick={() => setSwitchModalOpen(false)} className="text-gray-500 hover:text-gray-700">
                  <i className="fa-solid fa-xmark text-xl"></i>
                </button>
              </div>
              <p className="text-sm text-gray-500 mt-1">选择要打开的仓库</p>
            </div>

            <div className="p-4 max-h-80 overflow-auto">
              <div className="space-y-2">
                {vaults.map((vault) => (
                  <button
                    key={vault.path}
                    onClick={async () => {
                      try {
                        console.log("[VaultDialog] Switch modal: switching to:", vault.path);
                        await openVault(vault.path);
                        console.log("[VaultDialog] Switch modal: success");
                        window.dispatchEvent(new CustomEvent("vault-changed"));
                        await refreshData();
                        setSwitchModalOpen(false);
                      } catch (e) {
                        console.error("[VaultDialog] Switch modal: failed:", e);
                        alert("切换仓库失败: " + String(e));
                      }
                    }}
                    className={`w-full p-4 rounded-lg border text-left flex items-center gap-4 ${
                      currentVault?.path === vault.path
                        ? "border-blue-500 bg-blue-50"
                        : "border-gray-200 hover:border-blue-300 hover:bg-gray-50"
                    }`}
                  >
                    <div className={`w-10 h-10 rounded-lg flex items-center justify-center ${
                      currentVault?.path === vault.path ? "bg-blue-100" : "bg-purple-100"
                    }`}>
                      <i className={`fa-solid fa-database ${
                        currentVault?.path === vault.path ? "text-blue-600" : "text-purple-600"
                      }`}></i>
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="font-medium">{vault.name}</div>
                      <div className="text-sm text-gray-500 break-all">{vault.path}</div>
                    </div>
                    {currentVault?.path === vault.path && (
                      <i className="fa-solid fa-check text-blue-600"></i>
                    )}
                  </button>
                ))}

                <button
                  onClick={() => {
                    setSwitchModalOpen(false);
                    setImportModalOpen(true);
                  }}
                  className="w-full p-4 rounded-lg border-2 border-dashed border-emerald-300 hover:border-emerald-500 hover:bg-emerald-50/30 text-left flex items-center gap-4"
                >
                  <div className="w-10 h-10 bg-emerald-50 rounded-lg flex items-center justify-center">
                    <i className="fa-solid fa-file-import text-emerald-500"></i>
                  </div>
                  <div className="flex-1">
                    <div className="font-medium text-emerald-600">{t("vault.import.title")}</div>
                    <div className="text-sm text-gray-400">选择含仓库文件的目录登记回清单</div>
                  </div>
                </button>

                <button
                  onClick={() => {
                    setSwitchModalOpen(false);
                    setCreateModalOpen(true);
                  }}
                  className="w-full p-4 rounded-lg border-2 border-dashed border-gray-300 hover:border-blue-400 hover:bg-blue-50/30 text-left flex items-center gap-4"
                >
                  <div className="w-10 h-10 bg-gray-100 rounded-lg flex items-center justify-center">
                    <i className="fa-solid fa-folder-plus text-gray-400"></i>
                  </div>
                  <div className="flex-1">
                    <div className="font-medium text-gray-600">{t("vault.create.title")}</div>
                    <div className="text-sm text-gray-400">设置新的数据存储空间</div>
                  </div>
                </button>
              </div>
            </div>

            <div className="p-6 border-t border-gray-200">
              <button onClick={() => setSwitchModalOpen(false)} className="w-full px-4 py-2 border rounded-lg hover:bg-gray-50">
                {t("vault.delete.cancel")}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 导入已有仓库弹窗 */}
      {importModalOpen && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-[60]" onClick={() => importButtonState !== "importing" && setImportModalOpen(false)}>
          <div className="bg-white rounded-xl shadow-xl w-full max-w-lg max-h-[90vh] flex flex-col" onClick={(e) => e.stopPropagation()}>
            <div className="p-6 border-b border-gray-200 flex-shrink-0">
              <div className="flex items-center justify-between">
                <h2 className="text-xl font-semibold">{t("vault.import.title")}</h2>
                {importButtonState !== "importing" && (
                  <button onClick={() => setImportModalOpen(false)} className="text-gray-500 hover:text-gray-700">
                    <i className="fa-solid fa-xmark text-xl"></i>
                  </button>
                )}
              </div>
              <p className="text-sm text-gray-500 mt-1">选择已有仓库文件目录，校验齐全、有效后登记回仓库清单</p>
            </div>

            <div className="p-6 space-y-4 flex-1 overflow-y-auto">
              <div>
                <label className="block text-sm font-medium mb-2">仓库存放位置 <span className="text-red-500">*</span></label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={importVaultPath}
                    onChange={(e) => setImportVaultPath(e.target.value)}
                    placeholder="选择含 os_compass.db 和 .cryptokey 的目录"
                    className="flex-1 px-4 py-2 border rounded-lg bg-gray-50 focus:ring-2 focus:ring-emerald-500 focus:border-emerald-500"
                    disabled={importButtonState === "importing"}
                  />
                  <button
                    type="button"
                    onClick={async () => {
                      try {
                        const selected = await openDialog({
                          directory: true,
                          multiple: false,
                          title: t("vault.import.select"),
                        });
                        if (selected) {
                          setImportVaultPath(selected as string);
                          if (!importVaultName.trim()) {
                            const parts = (selected as string).replace(/\\/g, "/").split("/");
                            setImportVaultName(parts[parts.length - 1] || "");
                          }
                        }
                      } catch (e) {
                        console.error("Failed to open directory dialog:", e);
                      }
                    }}
                    disabled={importButtonState === "importing"}
                    className="px-4 py-2 border rounded-lg hover:bg-gray-50 disabled:opacity-50"
                  >
                    <i className="fa-solid fa-folder-open"></i>
                  </button>
                </div>
              </div>

              <div>
                <label className="block text-sm font-medium mb-2">{t("vault.create.name")} <span className="text-red-500">*</span></label>
                <input
                  type="text"
                  value={importVaultName}
                  onChange={(e) => setImportVaultName(e.target.value)}
                  placeholder="默认取目录名，可修改"
                  className="w-full px-4 py-2 border rounded-lg focus:ring-2 focus:ring-emerald-500 focus:border-emerald-500"
                  disabled={importButtonState === "importing"}
                />
              </div>

              <div className="bg-blue-50 border border-blue-200 rounded-lg p-3 flex items-start gap-2">
                <i className="fa-solid fa-circle-info text-blue-500 mt-0.5"></i>
                <div className="text-sm text-blue-700">
                  <p>将校验目录下的 <code className="bg-blue-100 px-1 rounded">os_compass.db</code> 和 <code className="bg-blue-100 px-1 rounded">.cryptokey</code> 文件齐全、有效后登记到仓库清单。</p>
                  <p className="mt-1 text-blue-500">导入后不会立即切换，可在清单中手动点"切换"。</p>
                </div>
              </div>

              {importError && (
                <div className="bg-red-50 border border-red-200 rounded-lg p-3">
                  <div className="flex items-start gap-2">
                    <i className="fa-solid fa-circle-xmark text-red-500 mt-0.5"></i>
                    <div>
                      <p className="text-sm text-red-700 font-medium">导入失败</p>
                      <p className="text-sm text-red-600 mt-1">{importError}</p>
                    </div>
                  </div>
                </div>
              )}
            </div>

            <div className="p-6 border-t border-gray-200 flex gap-3 flex-shrink-0">
              <button
                onClick={() => {
                  setImportModalOpen(false);
                  setImportVaultName("");
                  setImportVaultPath("");
                  setImportError(null);
                  setImportButtonState("idle");
                }}
                disabled={importButtonState === "importing"}
                className="flex-1 px-4 py-2 border rounded-lg hover:bg-gray-50 disabled:opacity-50"
              >
                {t("vault.delete.cancel")}
              </button>
              <button
                onClick={async () => {
                  if (!importVaultName.trim()) { alert("请输入仓库名称"); return; }
                  if (!importVaultPath.trim()) { alert("请选择仓库目录"); return; }
                  setImportButtonState("importing");
                  setImportError(null);
                  try {
                    await importVault(importVaultName.trim(), importVaultPath.trim());
                    await refreshData();
                    setImportModalOpen(false);
                    setImportVaultName("");
                    setImportVaultPath("");
                    setImportButtonState("idle");
                  } catch (err: any) {
                    setImportError(err.toString());
                    setImportButtonState("error");
                  }
                }}
                disabled={importButtonState === "importing" || !importVaultName.trim() || !importVaultPath.trim()}
                className="flex-1 px-4 py-2 bg-emerald-600 text-white rounded-lg hover:bg-emerald-700 disabled:opacity-50"
              >
                {importButtonState === "importing" ? (
                  <><i className="fa-solid fa-spinner fa-spin mr-2"></i>{t("vault.import.importing")}</>
                ) : (
                  <><i className="fa-solid fa-file-import mr-1"></i>{t("vault.import.importBtn")}</>
                )}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 删除确认弹窗 */}
      {deleteModalOpen && deleteTarget && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-[60]" onClick={() => setDeleteModalOpen(false)}>
          <div className="bg-white rounded-xl shadow-xl w-full max-w-md" onClick={(e) => e.stopPropagation()}>
            <div className="p-6 border-b border-gray-200">
              <div className="flex items-center gap-3">
                <div className="w-12 h-12 bg-red-100 rounded-full flex items-center justify-center">
                  <i className="fa-solid fa-triangle-exclamation text-red-600 text-xl"></i>
                </div>
                <div>
                  <h2 className="text-xl font-semibold">{t("vault.delete.title")}</h2>
                  <p className="text-sm text-gray-500">{t("vault.delete.warningSub")}</p>
                </div>
              </div>
            </div>

            <div className="p-6">
              <p className="text-gray-600 mb-4">{t("vault.delete.warning", { name: deleteTarget.name })}</p>

              <div className="space-y-3">
                <label className="flex items-start gap-3 p-3 border rounded-lg cursor-pointer hover:bg-gray-50">
                  <input
                    type="radio"
                    name="deleteMode"
                    value="index"
                    checked={deleteMode === "index"}
                    onChange={() => setDeleteMode("index")}
                    className="mt-1"
                  />
                  <div>
                    <div className="font-medium">仅从索引移除</div>
                    <div className="text-sm text-gray-500">数据库文件保留在原位置，可后续重新打开</div>
                  </div>
                </label>
                <label className="flex items-start gap-3 p-3 border border-red-200 rounded-lg cursor-pointer hover:bg-red-50">
                  <input
                    type="radio"
                    name="deleteMode"
                    value="permanent"
                    checked={deleteMode === "permanent"}
                    onChange={() => setDeleteMode("permanent")}
                    className="mt-1"
                  />
                  <div>
                    <div className="font-medium text-red-600">彻底删除</div>
                    <div className="text-sm text-red-500">删除数据库文件、加密密钥等所有仓库文件，不可恢复</div>
                  </div>
                </label>
              </div>
            </div>

            <div className="p-6 border-t border-gray-200 flex gap-3">
              <button
                onClick={() => {
                  setDeleteModalOpen(false);
                  setDeleteTarget(null);
                  setDeleteMode("index");
                }}
                className="flex-1 px-4 py-2 border rounded-lg hover:bg-gray-50"
              >
                {t("vault.delete.cancel")}
              </button>
              <button
                onClick={async () => {
                  try {
                    await deleteVault(deleteTarget.path, deleteMode === "permanent");
                    await refreshData();
                    setDeleteModalOpen(false);
                    setDeleteTarget(null);
                    setDeleteMode("index");
                  } catch (e) {
                    alert("删除失败: " + String(e));
                  }
                }}
                className={`flex-1 px-4 py-2 text-white rounded-lg ${
                  deleteMode === "permanent"
                    ? "bg-red-600 hover:bg-red-700"
                    : "bg-blue-600 hover:bg-blue-700"
                }`}
              >
                {deleteMode === "permanent" ? "彻底删除" : t("vault.delete.confirm")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
