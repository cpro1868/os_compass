import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { listVaults, createVault, openVault } from "../api";

interface Vault {
  name: string;
  path: string;
  project_count: number;
}

interface InitWizardProps {
  onComplete: () => void;
  isManual?: boolean;
}

type Step = "select" | "create" | "import" | "init" | "complete";

export function InitWizard({ onComplete, isManual = false }: InitWizardProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState<Step>("select");
  const [vaults, setVaults] = useState<Vault[]>([]);

  // 创建新仓库表单
  const [vaultName, setVaultName] = useState(t("initWizard.vaultName"));
  const [vaultPath, setVaultPath] = useState("");
  const [createError, setCreateError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  // 导入表单
  const [backupPath, setBackupPath] = useState("");
  const [targetPath, setTargetPath] = useState("");
  const [importing, setImporting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);
  const [backupValid, setBackupValid] = useState(false);

  // 初始化进度
  const [initProgress, setInitProgress] = useState({ percent: 0, steps: [] as string[] });

  useEffect(() => {
    loadVaults();
  }, []);

  const loadVaults = async () => {
    try {
      const vaultList = await listVaults();
      setVaults(vaultList);
    } catch (e) {
      console.error("Failed to load vaults:", e);
    }
  };

  const handleSelectVault = async (vault: Vault) => {
    try {
      await openVault(vault.path);
      window.dispatchEvent(new CustomEvent("vault-changed"));
      onComplete();
    } catch (e) {
      console.error("Failed to open vault:", e);
    }
  };

  const handleCreateVault = async () => {
    if (!vaultName.trim()) {
      setCreateError("请输入仓库名称");
      return;
    }
    if (!vaultPath.trim()) {
      setCreateError("请选择存储位置");
      return;
    }
    setCreating(true);
    setCreateError(null);

    try {
      await createVault(vaultName, vaultPath);
      setStep("init");
      await simulateInit();
    } catch (e) {
      setCreateError(String(e));
    } finally {
      setCreating(false);
    }
  };

  const simulateInit = async () => {
    const steps = [
      "创建仓库目录",
      "初始化数据库",
      "设置加密密钥",
      "完成初始化"
    ];

    for (let i = 0; i < steps.length; i++) {
      await new Promise(resolve => setTimeout(resolve, 500));
      setInitProgress({
        percent: Math.round(((i + 1) / steps.length) * 100),
        steps: steps.slice(0, i + 1)
      });
    }
    setStep("complete");
  };

  const handleBrowseCreate = async () => {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: "选择仓库存储位置"
    });
    if (selected) {
      setVaultPath(selected as string);
    }
  };

  const handleBrowseBackup = async () => {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: "选择备份目录"
    });
    if (selected) {
      setBackupPath(selected as string);
      // 验证备份
      try {
        await invoke("validate_backup", { path: selected });
        setBackupValid(true);
      } catch {
        setBackupValid(false);
      }
    }
  };

  const handleBrowseTarget = async () => {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: "选择导入目标位置"
    });
    if (selected) {
      setTargetPath(selected as string);
    }
  };

  const handleImport = async () => {
    if (!backupPath || !targetPath) return;
    setImporting(true);
    setImportError(null);

    try {
      await invoke("import_vault", { backupPath, targetPath });
      await openVault(targetPath);
      window.dispatchEvent(new CustomEvent("vault-changed"));
      onComplete();
    } catch (e) {
      setImportError(String(e));
    } finally {
      setImporting(false);
    }
  };

  const handleComplete = () => {
    window.dispatchEvent(new CustomEvent("vault-changed"));
    onComplete();
  };

  return (
    <div className="fixed inset-0 z-[200] bg-gradient-to-br from-slate-900 to-slate-800">
      {isManual && (
        <button
          onClick={onComplete}
          className="absolute top-6 left-6 w-10 h-10 rounded-lg bg-slate-700/50 hover:bg-slate-600 text-slate-300 hover:text-white flex items-center justify-center transition"
          title={t("initWizard.back")}
        >
          <i className="fa-solid fa-arrow-left"></i>
        </button>
      )}
      <div className="w-full max-w-5xl mx-auto p-6 pt-8">
        {/* Logo 和标题 */}
        <div className="text-center mb-6">
          <div className="inline-flex items-center justify-center w-14 h-14 bg-gradient-to-br from-blue-500 to-indigo-600 rounded-xl mb-4 shadow-xl">
            <i className="fa-solid fa-compass text-2xl text-white"></i>
          </div>
          <h1 className="text-2xl font-bold mb-2 bg-gradient-to-r from-white to-slate-400 bg-clip-text text-transparent">
            {t("initWizard.welcome")}
          </h1>
          <p className="text-slate-400 text-sm">{t("initWizard.subtitle")}</p>
        </div>

        {/* 步骤指示器 */}
        <div className="flex items-center justify-center mb-6">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-1.5">
              <div className={`w-6 h-6 rounded-full flex items-center justify-center text-xs font-medium ${
                step === "complete" ? "bg-emerald-500" :
                step !== "select" ? "bg-blue-500" : "bg-blue-500"
              }`}>
                <i className="fa-solid fa-check text-[10px]"></i>
              </div>
              <span className="text-xs">{t("initWizard.selectVault")}</span>
            </div>
            <div className="w-8 h-px bg-slate-600"></div>
            <div className="flex items-center gap-1.5">
              <div className={`w-6 h-6 rounded-full flex items-center justify-center text-xs font-medium ${
                step === "init" ? "bg-blue-500" :
                step === "complete" ? "bg-emerald-500" :
                "bg-slate-600"
              }`}>
                {step === "complete" ? (
                  <i className="fa-solid fa-check text-[10px]"></i>
                ) : (
                  <span>2</span>
                )}
              </div>
              <span className={`text-xs ${step === "init" ? "text-white" : "text-slate-500"}`}>{t("initWizard.init")}</span>
            </div>
            <div className="w-8 h-px bg-slate-600"></div>
            <div className="flex items-center gap-1.5">
              <div className={`w-6 h-6 rounded-full flex items-center justify-center text-xs font-medium ${
                step === "complete" ? "bg-emerald-500" : "bg-slate-600"
              }`}>
                {step === "complete" ? (
                  <i className="fa-solid fa-check text-[10px]"></i>
                ) : (
                  <span>3</span>
                )}
              </div>
              <span className={`text-xs ${step === "complete" ? "text-white" : "text-slate-500"}`}>{t("initWizard.complete")}</span>
            </div>
          </div>
        </div>

        {/* ========== 步骤 1: 选择/创建/导入仓库 ========== */}
        {step === "select" && (
          <div className="grid grid-cols-1 md:grid-cols-3 gap-5 mb-6">
            {/* 选项 1: 选择已有仓库 */}
            <div className="bg-slate-800/80 backdrop-blur rounded-2xl p-5 border border-slate-700 hover:border-blue-500/50 transition">
              <div className="w-12 h-12 bg-blue-600/20 rounded-lg flex items-center justify-center mb-3">
                <i className="fa-solid fa-folder-open text-blue-400 text-xl"></i>
              </div>
              <h3 className="text-lg font-semibold mb-1.5">{t("initWizard.selectExisting")}</h3>
              <p className="text-slate-400 text-sm mb-3">{t("initWizard.selectExistingDesc")}</p>
              <div className="space-y-2">
                {vaults.length === 0 ? (
                  <p className="text-slate-500 text-sm text-center py-4">{t("initWizard.noVaults")}</p>
                ) : (
                  vaults.map((vault) => (
                    <button
                      key={vault.path}
                      onClick={() => handleSelectVault(vault)}
                      className="w-full bg-slate-700/50 hover:bg-slate-700 transition rounded-lg p-3 flex items-center gap-3 text-left"
                    >
                      <i className="fa-solid fa-folder text-amber-400"></i>
                      <div className="flex-1 min-w-0">
                        <div className="font-medium text-sm truncate">{vault.name}</div>
                        <div className="text-xs text-slate-500 truncate">{vault.path}</div>
                      </div>
                      <i className="fa-solid fa-chevron-right text-slate-500"></i>
                    </button>
                  ))
                )}
              </div>
            </div>

            {/* 选项 2: 创建新仓库 */}
            <div className="bg-slate-800/80 backdrop-blur rounded-2xl p-5 border border-slate-700 hover:border-emerald-500/50 transition">
              <div className="w-12 h-12 bg-emerald-600/20 rounded-lg flex items-center justify-center mb-3">
                <i className="fa-solid fa-plus text-emerald-400 text-xl"></i>
              </div>
              <h3 className="text-lg font-semibold mb-1.5">{t("initWizard.createNew")}</h3>
              <p className="text-slate-400 text-sm mb-3">{t("initWizard.createNewDesc")}</p>
              <div className="space-y-3">
                <div>
                  <label className="block text-xs text-slate-400 mb-1">{t("initWizard.vaultName")}</label>
                  <input
                    type="text"
                    value={vaultName}
                    onChange={(e) => setVaultName(e.target.value)}
                    className="w-full bg-slate-700/50 border border-slate-600 focus:border-emerald-500 rounded-lg px-4 py-2 text-sm outline-none"
                    placeholder={t("initWizard.vaultName")}
                  />
                </div>
                <div>
                  <label className="block text-xs text-slate-400 mb-1">{t("initWizard.storageLocation")}</label>
                  <div className="relative">
                    <input
                      type="text"
                      value={vaultPath}
                      onChange={(e) => setVaultPath(e.target.value)}
                      className="w-full bg-slate-700/50 border border-slate-600 focus:border-emerald-500 rounded-lg px-4 py-2 text-sm pr-20 outline-none"
                      placeholder={t("initWizard.storageLocation") + "..."}
                    />
                    <button
                      onClick={handleBrowseCreate}
                      className="absolute right-2 top-1/2 -translate-y-1/2 px-3 py-1 bg-slate-600 hover:bg-slate-500 rounded text-xs transition"
                    >
                      {t("initWizard.browse")}
                    </button>
                  </div>
                </div>
                {createError && (
                  <p className="text-red-400 text-sm">{createError}</p>
                )}
                <button
                  onClick={handleCreateVault}
                  disabled={creating}
                  className="w-full bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 text-white font-medium py-2.5 rounded-lg transition flex items-center justify-center gap-2"
                >
                  {creating ? (
                    <i className="fa-solid fa-spinner fa-spin"></i>
                  ) : (
                    <>
                      <i className="fa-solid fa-arrow-right"></i>{t("initWizard.createAndContinue")}
                    </>
                  )}
                </button>
              </div>
            </div>

            {/* 选项 3: 从备份导入 */}
            <div className="bg-slate-800/80 backdrop-blur rounded-2xl p-5 border border-slate-700 hover:border-purple-500/50 transition">
              <div className="w-12 h-12 bg-purple-600/20 rounded-lg flex items-center justify-center mb-3">
                <i className="fa-solid fa-file-import text-purple-400 text-xl"></i>
              </div>
              <h3 className="text-lg font-semibold mb-1.5">{t("initWizard.importBackup")}</h3>
              <p className="text-slate-400 text-sm mb-3">{t("initWizard.importBackupDesc")}</p>
              <div className="space-y-3">
                <div>
                  <label className="block text-xs text-slate-400 mb-1">{t("initWizard.selectBackup")}</label>
                  <div className="relative">
                    <input
                      type="text"
                      value={backupPath}
                      readOnly
                      className="w-full bg-slate-700/50 border border-slate-600 rounded-lg px-4 py-2 text-sm pr-20 outline-none cursor-pointer"
                      placeholder={t("initWizard.selectBackup") + "..."}
                      onClick={handleBrowseBackup}
                    />
                    <button
                      onClick={handleBrowseBackup}
                      className="absolute right-2 top-1/2 -translate-y-1/2 px-3 py-1 bg-slate-600 hover:bg-slate-500 rounded text-xs transition"
                    >
                      {t("initWizard.browse")}
                    </button>
                  </div>
                </div>
                {backupValid && (
                  <div className="text-xs text-emerald-400">
                    <i className="fa-solid fa-check-circle mr-1"></i>
                    {t("initWizard.validationSuccess")}: os_compass.db & .cryptokey
                  </div>
                )}
                {backupValid && (
                  <div>
                    <label className="block text-xs text-slate-400 mb-1">{t("initWizard.targetLocation")}</label>
                    <div className="relative">
                      <input
                        type="text"
                        value={targetPath}
                        onChange={(e) => setTargetPath(e.target.value)}
                        className="w-full bg-slate-700/50 border border-slate-600 rounded-lg px-4 py-2 text-sm pr-20 outline-none"
                        placeholder={t("initWizard.targetLocation") + "..."}
                      />
                      <button
                        onClick={handleBrowseTarget}
                        className="absolute right-2 top-1/2 -translate-y-1/2 px-3 py-1 bg-slate-600 hover:bg-slate-500 rounded text-xs transition"
                      >
                        {t("initWizard.browse")}
                      </button>
                    </div>
                  </div>
                )}
                {importError && (
                  <p className="text-red-400 text-sm">{importError}</p>
                )}
                <button
                  onClick={handleImport}
                  disabled={!backupValid || !targetPath || importing}
                  className="w-full bg-purple-600 hover:bg-purple-500 disabled:opacity-50 text-white font-medium py-2.5 rounded-lg transition flex items-center justify-center gap-2"
                >
                  {importing ? (
                    <i className="fa-solid fa-spinner fa-spin"></i>
                  ) : (
                    <>
                      <i className="fa-solid fa-play"></i>{t("initWizard.startImport")}
                    </>
                  )}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* ========== 步骤 2: 初始化中 ========== */}
        {step === "init" && (
          <div className="bg-slate-800/80 backdrop-blur rounded-2xl p-8 border border-slate-700 max-w-xl mx-auto text-center">
            <div className="w-16 h-16 bg-emerald-600/20 rounded-2xl flex items-center justify-center mx-auto mb-6">
              <i className="fa-solid fa-database text-emerald-400 text-3xl"></i>
            </div>
            <h2 className="text-2xl font-bold mb-2">{t("initWizard.init")}</h2>
            <p className="text-slate-400 mb-6">正在为您创建新的仓库...</p>

            {/* 进度条 */}
            <div className="mb-6">
              <div className="flex justify-between text-xs text-slate-400 mb-2">
                <span>{t("initWizard.progress.creatingDir")}</span>
                <span>{initProgress.percent}%</span>
              </div>
              <div className="w-full bg-slate-700 rounded-full h-2">
                <div
                  className="bg-emerald-500 h-2 rounded-full transition-all duration-300"
                  style={{ width: `${initProgress.percent}%` }}
                ></div>
              </div>
            </div>

            {/* 步骤列表 */}
            <div className="space-y-3 text-sm text-left">
              {[t("initWizard.progress.creatingDir"), t("initWizard.progress.initDb"), t("initWizard.progress.settingKey"), t("initWizard.progress.complete")].map((stepName) => (
                <div
                  key={stepName}
                  className={`flex items-center gap-3 ${
                    initProgress.steps.includes(stepName) ? "text-emerald-400" : "text-slate-400"
                  }`}
                >
                  <i className={`fa-solid w-4 ${
                    initProgress.steps.includes(stepName) ? "fa-check-circle" : "fa-circle"
                  }`}></i>
                  <span>{stepName}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* ========== 步骤 3: 完成 ========== */}
        {step === "complete" && (
          <div className="text-center">
            <div className="w-20 h-20 bg-emerald-600/20 rounded-full flex items-center justify-center mx-auto mb-6">
              <i className="fa-solid fa-check text-emerald-400 text-4xl"></i>
            </div>
            <h2 className="text-2xl font-bold mb-2">{t("initWizard.completed")}</h2>
            <p className="text-slate-400 mb-8">您的仓库已准备就绪</p>
            <button
              onClick={handleComplete}
              className="px-8 py-3 bg-blue-600 hover:bg-blue-500 text-white font-medium rounded-xl transition flex items-center gap-2 mx-auto"
            >
              <i className="fa-solid fa-rocket"></i>
              {t("initWizard.startUsing")}
            </button>
          </div>
        )}

        {/* 底部说明 */}
        <div className="text-center text-slate-500 text-sm space-y-2 mt-8">
          <p className="flex items-center justify-center gap-2">
            <i className="fa-solid fa-shield-halved text-emerald-400"></i>
            您的数据将安全存储在本地，不会上传至任何服务器
          </p>
          <p className="text-xs text-slate-600">
            仓库是存储您开源项目信息的文件夹，您可以创建多个仓库来分类管理
          </p>
        </div>
      </div>
    </div>
  );
}
