import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import i18n from "../locales/i18n";
import type { SystemVariable, SourcePlugin } from "../types";
import { getSettings, saveSettings } from "../api";

interface AppSettings {
  data_dir: string;
  theme: string;
  default_language: string;
  auto_translate_readme: boolean;
  auto_translate_report: boolean;
  download_path: string;
  default_editor: string;
  llm_provider: string;
  llm_api_base: string;
  llm_api_key: string;
  llm_model: string;
  llm_available_models: string[];
  llm_proxy_enabled: boolean;
  llm_proxy_protocol: string;
  llm_proxy_host: string;
  llm_proxy_port: number;
  llm_proxy_username: string;
  llm_proxy_password: string;
  proxy_enabled: boolean;
  proxy_protocol: string;
  proxy_host: string;
  proxy_port: number;
  proxy_username: string;
  proxy_password: string;
  crawler_enabled: boolean;
  crawler_api_url: string;
  readme_update_frequency: string;
  translate_engine: "auto" | "google" | "llm";
  google_api_key: string;
  google_proxy_enabled: boolean;
  google_proxy_protocol: string;
  google_proxy_host: string;
  google_proxy_port: number;
  google_proxy_username: string;
  google_proxy_password: string;
}

interface SettingsDialogProps {
  open: boolean;
  onClose: () => void;
  onThemeChange?: (theme: string) => void;
}

type TabId = "general" | "download" | "llm" | "translation" | "variables" | "extensions" | "plugins";

interface FeaturePlugin {
  id: string;
  name: string;
  plugin_type: string;
  enabled: boolean;
  config: string | null;
  version: string;
  db_mode: string;
  db_path_template: string | null;
}

export function SettingsDialog({ open, onClose, onThemeChange }: SettingsDialogProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<TabId>("general");
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<"success" | "failed" | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [variables, setVariables] = useState<SystemVariable[]>([]);
  const [extensions, setExtensions] = useState<SourcePlugin[]>([]);
  const [featurePlugins, setFeaturePlugins] = useState<FeaturePlugin[]>([]);
  const [varModalOpen, setVarModalOpen] = useState(false);
  const [varForm, setVarForm] = useState({ key: "", value: "", isSecret: false });
  const [showVarValue, setShowVarValue] = useState(false);
  const [editingVarKey, setEditingVarKey] = useState<string | null>(null);
  const [fetchingModels, setFetchingModels] = useState(false);
  const [modelError, setModelError] = useState<string | null>(null);

  const [settings, setSettings] = useState<AppSettings>({
    data_dir: "",
    theme: "dark",
    default_language: "zh-CN",
    auto_translate_readme: true,
    auto_translate_report: true,
    download_path: "",
    default_editor: "code",
    llm_provider: "openai",
    llm_api_base: "https://api.openai.com/v1",
    llm_api_key: "",
    llm_model: "",
    llm_available_models: [],
    llm_proxy_enabled: false,
    llm_proxy_protocol: "http",
    llm_proxy_host: "",
    llm_proxy_port: 0,
    llm_proxy_username: "",
    llm_proxy_password: "",
    proxy_enabled: false,
    proxy_protocol: "http",
    proxy_host: "",
    proxy_port: 0,
    proxy_username: "",
    proxy_password: "",
    crawler_enabled: false,
    crawler_api_url: "",
    readme_update_frequency: "smart",
    translate_engine: "auto",
    google_api_key: "",
    google_proxy_enabled: false,
    google_proxy_protocol: "http",
    google_proxy_host: "",
    google_proxy_port: 0,
    google_proxy_username: "",
    google_proxy_password: "",
  });

  useEffect(() => {
    if (open) {
      getSettings()
        .then((s) => setSettings(s as any))
        .catch(console.error);
      invoke<SystemVariable[]>("get_system_variables")
        .then(setVariables)
        .catch(console.error);
      invoke<SourcePlugin[]>("list_extensions")
        .then(setExtensions)
        .catch(console.error);
      invoke<FeaturePlugin[]>("list_feature_plugins")
        .then(setFeaturePlugins)
        .catch(console.error);
    }
  }, [open]);

  const validateProxy = (enabled: boolean, host: string, port: number, name: string): string | null => {
    if (!enabled) return null;
    if (!host || !host.trim()) return t("settings.proxyEmptyError", { name });
    if (!port || port <= 0) return t("settings.proxyPortError", { name });
    return null;
  };

  const handleSave = async () => {
    let err = validateProxy(settings.proxy_enabled, settings.proxy_host, settings.proxy_port, t("settings.downloadProxy"));
    if (err) { alert(err); return; }
    err = validateProxy(settings.llm_proxy_enabled, settings.llm_proxy_host, settings.llm_proxy_port, t("settings.llmProxy"));
    if (err) { alert(err); return; }
    err = validateProxy(settings.google_proxy_enabled, settings.google_proxy_host, settings.google_proxy_port, t("settings.googleProxyName"));
    if (err) { alert(err); return; }

    setSaving(true);
    setSaved(false);
    try {
      await saveSettings(settings as any);
      if (settings.default_language !== i18n.language) {
        i18n.changeLanguage(settings.default_language);
      }
      setSaved(true);
      setTimeout(() => setSaved(false), 3000);
    } catch (e) {
      alert(`${t("settings.saveFailed")}: ${String(e)}`);
    } finally {
      setSaving(false);
    }
  };

  const handleTestLLM = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      await invoke("analyze_project", { id: -1 });
      setTestResult("success");
    } catch {
      setTestResult("failed");
    } finally {
      setTesting(false);
    }
  };

  const tabs = [
    { id: "general" as TabId, label: t("settings.tabs.general") },
    { id: "download" as TabId, label: t("settings.tabs.download") },
    { id: "llm" as TabId, label: t("settings.tabs.llm") },
    { id: "translation" as TabId, label: t("settings.tabs.translation") },
    { id: "variables" as TabId, label: t("settings.tabs.variables") },
    { id: "extensions" as TabId, label: t("settings.tabs.extensions") },
    { id: "plugins" as TabId, label: t("settings.tabs.plugins") },
  ];

  if (!open) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-xl shadow-xl w-full max-w-3xl max-h-[90vh] overflow-hidden flex flex-col dark:bg-gray-800">
        <div className="p-6 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-xl font-bold">{t("settings.title")}</h2>
            <button
              onClick={onClose}
              className="w-8 h-8 flex items-center justify-center text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg dark:text-gray-500 dark:hover:text-gray-300 dark:hover:bg-gray-700"
            >
              <i className="fa-solid fa-xmark text-lg"></i>
            </button>
          </div>
          <div className="border-b border-gray-200 flex dark:border-gray-700">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`px-6 py-3 border-b-2 ${
                  activeTab === tab.id
                    ? "border-blue-600 text-blue-600 dark:border-blue-400 dark:text-blue-400"
                    : "border-transparent text-gray-500 hover:text-blue-600 dark:text-gray-400 dark:hover:text-blue-400"
                }`}
              >
                {tab.label}
              </button>
            ))}
          </div>
        </div>

        <div className="flex-1 p-6 overflow-y-auto">
          {activeTab === "general" && (
            <div className="space-y-6">
              <div>
                <h3 className="font-medium mb-4">{t("settings.appearance")}</h3>
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium mb-1">{t("settings.theme")}</label>
                    <select
                      value={settings.theme}
                      onChange={(e) => {
                        setSettings({ ...settings, theme: e.target.value });
                        onThemeChange?.(e.target.value);
                      }}
                      className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                    >
                      <option value="dark">{t("settings.themeDark")}</option>
                      <option value="light">{t("settings.themeLight")}</option>
                      <option value="system">{t("settings.themeSystem")}</option>
                    </select>
                  </div>
                  <div>
                    <label className="block text-sm font-medium mb-1">{t("settings.language")}</label>
                    <select
                      value={settings.default_language}
                      onChange={(e) => setSettings({ ...settings, default_language: e.target.value })}
                      className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                    >
                      <option value="zh-CN">{t("settings.langZh")}</option>
                      <option value="en-US">English</option>
                    </select>
                  </div>
                </div>
              </div>
            </div>
          )}

          {activeTab === "download" && (
            <div className="space-y-6">
              <div>
                <h3 className="font-medium mb-4">{t("settings.download")}</h3>
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium mb-1">{t("settings.sourceDir")}</label>
                    <input
                      type="text"
                      value={settings.download_path}
                      onChange={(e) => setSettings({ ...settings, download_path: e.target.value })}
                      placeholder="D:\OpenSource_Workspace"
                      className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium mb-1">{t("settings.editor")}</label>
                    <select
                      value={settings.default_editor}
                      onChange={(e) => setSettings({ ...settings, default_editor: e.target.value })}
                      className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                    >
                      <option value="code">VS Code</option>
                      <option value="cursor">Cursor</option>
                      <option value="webstorm">WebStorm</option>
                      <option value="sublime">Sublime Text</option>
                    </select>
                  </div>
                </div>
              </div>

              <div className="border-t border-gray-100 pt-6 dark:border-gray-700">
                <h3 className="font-medium mb-4">{t("settings.proxy")}</h3>
                <div className="space-y-4">
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={settings.proxy_enabled}
                      onChange={(e) => setSettings({ ...settings, proxy_enabled: e.target.checked })}
                      className="rounded"
                    />
                    <span className="text-sm">{t("settings.enableProxy")}</span>
                  </label>
                  {settings.proxy_enabled && (
                    <>
                      <div className="grid grid-cols-2 gap-4">
                        <div>
                          <label className="block text-sm font-medium mb-1">{t("settings.protocol")}</label>
                          <select
                            value={settings.proxy_protocol}
                            onChange={(e) => setSettings({ ...settings, proxy_protocol: e.target.value })}
                            className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                          >
                            <option value="http">HTTP</option>
                            <option value="https">HTTPS</option>
                            <option value="socks5">SOCKS5</option>
                          </select>
                        </div>
                        <div>
                          <label className="block text-sm font-medium mb-1">{t("settings.port")}</label>
                          <input
                            type="number"
                            value={settings.proxy_port || ""}
                            onChange={(e) => setSettings({ ...settings, proxy_port: parseInt(e.target.value) || 0 })}
                            placeholder="7890"
                            className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                          />
                        </div>
                      </div>
                      <div>
                        <label className="block text-sm font-medium mb-1">{t("settings.address")}</label>
                        <input
                          type="text"
                          value={settings.proxy_host}
                          onChange={(e) => setSettings({ ...settings, proxy_host: e.target.value })}
                          placeholder="127.0.0.1"
                          className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                        />
                      </div>
                      <div className="grid grid-cols-2 gap-4">
                        <div>
                          <label className="block text-sm font-medium mb-1">{t("settings.username")}</label>
                          <input
                            type="text"
                            value={settings.proxy_username}
                            onChange={(e) => setSettings({ ...settings, proxy_username: e.target.value })}
                            placeholder="username"
                            className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                          />
                        </div>
                        <div>
                          <label className="block text-sm font-medium mb-1">{t("settings.password")}</label>
                          <input
                            type="password"
                            value={settings.proxy_password}
                            onChange={(e) => setSettings({ ...settings, proxy_password: e.target.value })}
                            placeholder="password"
                            className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                          />
                        </div>
                      </div>
                    </>
                  )}
                </div>
              </div>
            </div>
          )}

          {activeTab === "llm" && (
            <div className="space-y-6">
              <div>
                <h3 className="font-medium mb-4">{t("settings.llmConfig")}</h3>
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium mb-1">{t("settings.llmProvider")}</label>
                    <select
                      value={settings.llm_provider}
                      onChange={(e) => setSettings({ ...settings, llm_provider: e.target.value })}
                      className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                    >
                      <option value="openai">OpenAI</option>
                      <option value="deepseek">DeepSeek</option>
                      <option value="anthropic">Anthropic</option>
                      <option value="ollama">Ollama（本地）</option>
                      <option value="custom">{t("settings.custom")}</option>
                    </select>
                  </div>
                  <div>
                    <label className="block text-sm font-medium mb-1">{t("settings.apiBase")}</label>
                    <input
                      type="text"
                      value={settings.llm_api_base}
                      onChange={(e) => setSettings({ ...settings, llm_api_base: e.target.value })}
                      placeholder="https://api.openai.com/v1"
                      className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                    />
                    <p className="text-xs text-gray-500 mt-1 dark:text-gray-400">{t("settings.apiBaseTip")}</p>
                  </div>
                  <div>
                    <label className="block text-sm font-medium mb-1">{t("settings.apiKey")}</label>
                    <input
                      type="password"
                      value={settings.llm_api_key}
                      onChange={(e) => setSettings({ ...settings, llm_api_key: e.target.value })}
                      placeholder="sk-..."
                      className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                    />
                    <p className="text-xs text-gray-500 mt-1 dark:text-gray-400">{t("settings.apiKeyTip")}</p>
                  </div>
                  <div>
                    <label className="block text-sm font-medium mb-1">
                      {t("settings.model")}
                      <button
                        type="button"
                        onClick={async () => {
                          if (!settings.llm_api_base || !settings.llm_api_key) {
                            setModelError(t("settings.fetchModelsError"));
                            return;
                          }
                          setFetchingModels(true);
                          setModelError(null);
                          try {
                            const models = await invoke<string[]>("list_available_models", {
                              provider: settings.llm_provider,
                              apiBase: settings.llm_api_base,
                              apiKey: settings.llm_api_key,
                            });
                            setSettings((s) => ({ ...s, llm_available_models: models }));
                            if (models.length > 0) {
                              setSettings((s) => ({ ...s, llm_model: models[0] }));
                            }
                          } catch (e) {
                            setModelError(`${t("settings.fetchModelsFailed")}: ${e}`);
                          } finally {
                            setFetchingModels(false);
                          }
                        }}
                        disabled={fetchingModels || !settings.llm_api_base || !settings.llm_api_key}
                        className="ml-2 px-2 py-1 text-xs bg-gray-100 hover:bg-gray-200 rounded disabled:opacity-50 dark:bg-gray-700 dark:hover:bg-gray-600"
                      >
                        {fetchingModels ? t("settings.fetchingModels") : t("settings.fetchModels")}
                      </button>
                    </label>
                    {(settings.llm_available_models || []).length > 0 ? (
                      <div className="space-y-2">
                        <div className="flex gap-2">
                          <select
                            value={settings.llm_available_models.includes(settings.llm_model) ? settings.llm_model : "__custom__"}
                            onChange={(e) => {
                              if (e.target.value === "__custom__") {
                                setSettings({ ...settings, llm_model: "" });
                              } else {
                                setSettings({ ...settings, llm_model: e.target.value });
                              }
                            }}
                            className="flex-1 px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                          >
                            <option value="__custom__">{t("settings.manualInput")}</option>
                            {(settings.llm_available_models || []).map((m) => (
                              <option key={m} value={m}>{m}</option>
                            ))}
                          </select>
                          {!settings.llm_available_models.includes(settings.llm_model) && (
                            <input
                              type="text"
                              value={settings.llm_model}
                              onChange={(e) => setSettings({ ...settings, llm_model: e.target.value })}
                              placeholder={t("settings.manualInputPlaceholder")}
                              className="flex-1 px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                            />
                          )}
                        </div>
                        <p className="text-xs text-green-600">
                          {t("settings.modelsFetched", { count: settings.llm_available_models.length })}
                        </p>
                      </div>
                    ) : (
                      <input
                        type="text"
                        value={settings.llm_model}
                        onChange={(e) => setSettings({ ...settings, llm_model: e.target.value })}
                        placeholder="gpt-4o"
                        className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                      />
                    )}
                    {modelError && (
                      <p className="text-xs text-red-500 mt-1">{modelError}</p>
                    )}
                    <p className="text-xs text-gray-500 mt-1 dark:text-gray-400">{t("settings.modelTip")}</p>
                  </div>
                  <div className="flex items-center gap-3">
                    <button
                      onClick={handleTestLLM}
                      disabled={testing || !settings.llm_api_key}
                      className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
                    >
                      {testing ? t("settings.testing") : t("settings.testConnection")}
                    </button>
                    {testResult === "success" && (
                      <span className="text-sm text-green-600">
                        <i className="fa-solid fa-check-circle mr-1"></i>
                        {t("settings.testSuccess")}
                      </span>
                    )}
                    {testResult === "failed" && (
                      <span className="text-sm text-red-600">
                        <i className="fa-solid fa-times-circle mr-1"></i>
                        {t("settings.testFailed")}
                      </span>
                    )}
                  </div>
                </div>

                <div className="border-t border-gray-100 pt-6 mt-6 dark:border-gray-700">
                  <h3 className="font-medium mb-4">{t("settings.llmProxySettings")}</h3>
                  <div className="space-y-4">
                    <label className="flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={settings.llm_proxy_enabled}
                        onChange={(e) => setSettings({ ...settings, llm_proxy_enabled: e.target.checked })}
                        className="rounded"
                      />
                      <span className="text-sm">{t("settings.enableProxy")}</span>
                    </label>
                    {settings.llm_proxy_enabled && (
                      <>
                        <div className="grid grid-cols-2 gap-4">
                          <div>
                            <label className="block text-sm font-medium mb-1">{t("settings.protocol")}</label>
                            <select
                              value={settings.llm_proxy_protocol}
                              onChange={(e) => setSettings({ ...settings, llm_proxy_protocol: e.target.value })}
                              className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                            >
                              <option value="http">HTTP</option>
                              <option value="https">HTTPS</option>
                              <option value="socks5">SOCKS5</option>
                            </select>
                          </div>
                          <div>
                            <label className="block text-sm font-medium mb-1">{t("settings.port")}</label>
                            <input
                              type="number"
                              value={settings.llm_proxy_port || ""}
                              onChange={(e) => setSettings({ ...settings, llm_proxy_port: parseInt(e.target.value) || 0 })}
                              placeholder="7890"
                              className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                            />
                          </div>
                        </div>
                        <div>
                            <label className="block text-sm font-medium mb-1">{t("settings.proxyAddress")}</label>
                            <input
                              type="text"
                              value={settings.llm_proxy_host}
                            onChange={(e) => setSettings({ ...settings, llm_proxy_host: e.target.value })}
                            placeholder="127.0.0.1"
                            className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                          />
                        </div>
                        <div className="grid grid-cols-2 gap-4">
                          <div>
                            <label className="block text-sm font-medium mb-1">{t("settings.username")}</label>
                            <input
                              type="text"
                              value={settings.llm_proxy_username}
                              onChange={(e) => setSettings({ ...settings, llm_proxy_username: e.target.value })}
                              placeholder="username"
                              className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                            />
                          </div>
                          <div>
                            <label className="block text-sm font-medium mb-1">{t("settings.password")}</label>
                            <input
                              type="password"
                              value={settings.llm_proxy_password}
                              onChange={(e) => setSettings({ ...settings, llm_proxy_password: e.target.value })}
                              placeholder="password"
                              className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                            />
                          </div>
                        </div>
                      </>
                    )}
                  </div>
                </div>
              </div>

              <div className="border-t border-gray-100 pt-6 dark:border-gray-700">
                <h3 className="font-medium mb-4">{t("settings.translatePref")}</h3>
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium mb-1">{t("settings.defaultLang")}</label>
                    <select
                      value={settings.default_language}
                      onChange={(e) => setSettings({ ...settings, default_language: e.target.value })}
                      className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                    >
                      <option value="zh-CN">{t("settings.langZh")}</option>
                      <option value="en-US">English</option>
                      <option value="ja-JP">日本語</option>
                    </select>
                    <p className="text-xs text-gray-500 mt-1 dark:text-gray-400">{t("settings.defaultLangTip")}</p>
                  </div>
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={settings.auto_translate_readme}
                      onChange={(e) => setSettings({ ...settings, auto_translate_readme: e.target.checked })}
                      className="rounded"
                    />
                    <span className="text-sm">{t("settings.autoTranslateReadme")}</span>
                  </label>
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={settings.auto_translate_report}
                      onChange={(e) => setSettings({ ...settings, auto_translate_report: e.target.checked })}
                      className="rounded"
                    />
                    <span className="text-sm">{t("settings.autoTranslateReport")}</span>
                  </label>
                </div>
              </div>

              <div className="border-t border-gray-100 pt-6 dark:border-gray-700">
                <h3 className="font-medium mb-4">{t("settings.crawler")}</h3>
                <div className="space-y-4">
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={settings.crawler_enabled}
                      onChange={(e) => setSettings({ ...settings, crawler_enabled: e.target.checked })}
                      className="rounded"
                    />
                    <span className="text-sm">{t("settings.enableCrawler")}</span>
                  </label>
                  {settings.crawler_enabled && (
                    <div>
                      <label className="block text-sm font-medium mb-1">{t("settings.crawlerApi")}</label>
                      <input
                        type="text"
                        value={settings.crawler_api_url}
                        onChange={(e) => setSettings({ ...settings, crawler_api_url: e.target.value })}
                        placeholder="http://localhost:8080/crawl"
                        className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                      />
                      <p className="text-xs text-gray-500 mt-1 dark:text-gray-400">{t("settings.crawlerApiTip")}</p>
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}

          {activeTab === "translation" && (
            <div className="space-y-6">
              <div>
                <h3 className="font-medium mb-4">{t("settings.translateEngine")}</h3>
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium mb-1">{t("settings.engineSelect")}</label>
                    <select
                      value={settings.translate_engine || "auto"}
                      onChange={(e) => setSettings({ ...settings, translate_engine: e.target.value as "auto" | "google" | "llm" })}
                      className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                    >
                      <option value="auto">{t("settings.engineAuto")}</option>
                      <option value="google">{t("settings.engineGoogle")}</option>
                      <option value="llm">{t("settings.engineLlm")}</option>
                    </select>
                    <p className="text-xs text-gray-500 mt-1 dark:text-gray-400">
                      {t("settings.engineAutoTip")}
                    </p>
                  </div>

                  <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 dark:bg-blue-950 dark:border-blue-800">
                    <h4 className="font-medium text-blue-800 mb-2 dark:text-blue-300">{t("settings.engineGuideTitle")}</h4>
                    <ul className="text-sm text-blue-700 space-y-1 dark:text-blue-300">
                      <li>{t("settings.engineGuideGoogle")}</li>
                      <li>{t("settings.engineGuideLlm")}</li>
                      <li>{t("settings.engineGuideAuto")}</li>
                    </ul>
                  </div>

                  <div className="border-t border-gray-200 pt-4 dark:border-gray-700">
                    <h4 className="font-medium mb-3">{t("settings.googleSettings")}</h4>
                    <div className="space-y-4">
                      <div>
                        <label className="block text-sm font-medium mb-1">{t("settings.googleApiKey")}</label>
                        <input
                          type="password"
                          value={settings.google_api_key || ""}
                          onChange={(e) => setSettings({ ...settings, google_api_key: e.target.value })}
                          placeholder="AIza..."
                          className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                        />
                        <p className="text-xs text-gray-500 mt-1 dark:text-gray-400">
                          {t("settings.googleApiKeyTip")}
                        </p>
                      </div>

                      <label className="flex items-center gap-2">
                        <input
                          type="checkbox"
                          checked={settings.google_proxy_enabled || false}
                          onChange={(e) => setSettings({ ...settings, google_proxy_enabled: e.target.checked })}
                          className="rounded"
                        />
                        <span className="text-sm">{t("settings.googleProxy")}</span>
                      </label>

                      {settings.google_proxy_enabled && (
                        <div className="space-y-4 pl-6">
                          <div className="grid grid-cols-2 gap-4">
                            <div>
                              <label className="block text-sm font-medium mb-1">{t("settings.proxyProtocol")}</label>
                              <select
                                value={settings.google_proxy_protocol || "http"}
                                onChange={(e) => setSettings({ ...settings, google_proxy_protocol: e.target.value })}
                                className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                              >
                                <option value="http">HTTP</option>
                                <option value="https">HTTPS</option>
                                <option value="socks5">SOCKS5</option>
                              </select>
                            </div>
                            <div>
                              <label className="block text-sm font-medium mb-1">{t("settings.port")}</label>
                              <input
                                type="number"
                                value={settings.google_proxy_port || ""}
                                onChange={(e) => setSettings({ ...settings, google_proxy_port: parseInt(e.target.value) || 0 })}
                                placeholder="7890"
                                className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                              />
                            </div>
                          </div>
                          <div>
                            <label className="block text-sm font-medium mb-1">{t("settings.proxyAddress")}</label>
                            <input
                              type="text"
                              value={settings.google_proxy_host || ""}
                              onChange={(e) => setSettings({ ...settings, google_proxy_host: e.target.value })}
                              placeholder="127.0.0.1"
                              className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                            />
                          </div>
                          <div className="grid grid-cols-2 gap-4">
                            <div>
                              <label className="block text-sm font-medium mb-1">{t("settings.username")}</label>
                              <input
                                type="text"
                                value={settings.google_proxy_username || ""}
                                onChange={(e) => setSettings({ ...settings, google_proxy_username: e.target.value })}
                                placeholder="username"
                                className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                              />
                            </div>
                            <div>
                              <label className="block text-sm font-medium mb-1">{t("settings.password")}</label>
                              <input
                                type="password"
                                value={settings.google_proxy_password || ""}
                                onChange={(e) => setSettings({ ...settings, google_proxy_password: e.target.value })}
                                placeholder="password"
                                className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                              />
                            </div>
                          </div>
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              </div>

              <div className="border-t border-gray-200 pt-6 dark:border-gray-700">
                <h3 className="font-medium mb-4">{t("settings.translatePref")}</h3>
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium mb-1">{t("settings.defaultTargetLang")}</label>
                    <select
                      value={settings.default_language}
                      onChange={(e) => setSettings({ ...settings, default_language: e.target.value })}
                      className="w-full px-3 py-2 border rounded-lg dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                    >
                      <option value="zh-CN">中文（简体）</option>
                      <option value="en-US">English</option>
                      <option value="ja-JP">日本語</option>
                      <option value="ko-KR">한국어</option>
                    </select>
                    <p className="text-xs text-gray-500 mt-1 dark:text-gray-400">
                      {t("settings.defaultTargetLangTip")}
                    </p>
                  </div>
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={settings.auto_translate_readme}
                      onChange={(e) => setSettings({ ...settings, auto_translate_readme: e.target.checked })}
                      className="rounded"
                    />
                    <span className="text-sm">{t("settings.autoTranslateReadme")}</span>
                  </label>
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={settings.auto_translate_report}
                      onChange={(e) => setSettings({ ...settings, auto_translate_report: e.target.checked })}
                      className="rounded"
                    />
                    <span className="text-sm">{t("settings.autoTranslateReportShort")}</span>
                  </label>
                </div>
              </div>
            </div>
          )}

          {activeTab === "variables" && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="font-medium">{t("settings.systemVariables")}</h3>
                  <p className="text-sm text-gray-500 dark:text-gray-400">{t("settings.systemVariablesTip")}</p>
                </div>
                <button
                  onClick={() => {
                    setEditingVarKey(null);
                    setVarForm({ key: "", value: "", isSecret: false });
                    setShowVarValue(false);
                    setVarModalOpen(true);
                  }}
                  className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                >
                  <i className="fa-solid fa-plus mr-2"></i>
                  {t("settings.addVariable")}
                </button>
              </div>

              <div className="border rounded-lg overflow-hidden dark:border-gray-700">
                <table className="w-full">
                  <thead className="bg-gray-50 border-b dark:bg-gray-900 dark:border-gray-700">
                    <tr>
                      <th className="text-left px-4 py-3 text-sm font-medium text-gray-600 dark:text-gray-400">{t("settings.variableKey")}</th>
                      <th className="text-left px-4 py-3 text-sm font-medium text-gray-600 dark:text-gray-400">{t("settings.variableValue")}</th>
                      <th className="text-center px-4 py-3 text-sm font-medium text-gray-600 dark:text-gray-400">{t("settings.variableType")}</th>
                      <th className="text-center px-4 py-3 text-sm font-medium text-gray-600 dark:text-gray-400">{t("common.actions")}</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y dark:divide-gray-700">
                    {variables.length === 0 && (
                      <tr>
                        <td colSpan={4} className="text-center py-8 text-gray-500 dark:text-gray-400">
                          {t("settings.noVariables")}
                        </td>
                      </tr>
                    )}
                    {variables.map((v) => (
                      <tr key={v.key} className="hover:bg-gray-50 dark:hover:bg-gray-700">
                        <td className="px-4 py-3 font-mono text-sm">{v.key}</td>
                        <td className="px-4 py-3 font-mono text-sm text-gray-500 dark:text-gray-400">
                          {v.isSecret ? "••••••••" : v.value || "-"}
                        </td>
                        <td className="px-4 py-3 text-center">
                          {v.isSecret ? (
                            <span className="px-2 py-1 bg-orange-100 text-orange-700 text-xs rounded dark:bg-orange-900 dark:text-orange-300">{t("settings.secret")}</span>
                          ) : (
                            <span className="px-2 py-1 bg-gray-100 text-gray-600 text-xs rounded dark:bg-gray-700 dark:text-gray-300">{t("settings.normal")}</span>
                          )}
                        </td>
                        <td className="px-4 py-3 text-center">
                          <button
                            onClick={() => {
                              setEditingVarKey(v.key);
                              setVarForm({ key: v.key, value: v.value || "", isSecret: v.isSecret });
                              setShowVarValue(false);
                              setVarModalOpen(true);
                            }}
                            className="text-blue-600 hover:underline mr-3"
                          >
                            {t("settings.editVariable")}
                          </button>
                          <button
                            onClick={() => {
                              if (window.confirm(t("settings.confirmDeleteVar"))) {
                                invoke("delete_system_variable", { key: v.key }).then(() => {
                                  setVariables(variables.filter((x) => x.key !== v.key));
                                });
                              }
                            }}
                            className="text-red-600 hover:underline"
                          >
                            {t("common.delete")}
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {activeTab === "extensions" && (
            <div className="space-y-4">
              <h3 className="font-medium">{t("settings.installedExtensions")}</h3>
              <div className="space-y-3">
                {extensions.map((ext) => (
                  <div key={ext.id} className="bg-gray-50 rounded-xl border p-4 dark:bg-gray-900 dark:border-gray-700">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <div className="w-10 h-10 bg-gray-800 rounded-lg flex items-center justify-center text-white dark:bg-gray-700">
                          {ext.id === "github" && <i className="fa-brands fa-github text-xl"></i>}
                          {ext.id === "gitee" && <i className="fa-solid fa-code text-xl"></i>}
                          {ext.id === "crawler" && <i className="fa-solid fa-spider text-xl"></i>}
                        </div>
                        <div>
                          <div className="flex items-center gap-2">
                            <span className="font-medium">{ext.name}</span>
                            <span className="text-xs bg-blue-100 text-blue-700 px-2 py-0.5 rounded dark:bg-blue-900 dark:text-blue-300">
                              v{ext.version || "1.0.0"}
                            </span>
                          </div>
                          <p className="text-sm text-gray-500 font-mono dark:text-gray-400">{ext.pluginClass}</p>
                        </div>
                      </div>
                      <div className="flex items-center gap-3">
                        <label className="relative inline-flex items-center cursor-pointer">
                          <input
                            type="checkbox"
                            checked={ext.enabled}
                            onChange={(e) => {
                              invoke("set_extension_enabled", { id: ext.id, enabled: e.target.checked });
                              setExtensions(extensions.map((x) => x.id === ext.id ? { ...x, enabled: e.target.checked } : x));
                            }}
                            className="sr-only peer"
                          />
                          <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600 dark:bg-gray-700 dark:after:border-gray-600"></div>
                        </label>
                      </div>
                    </div>
                    {ext.requiredVariables && ext.requiredVariables.length > 0 && (
                      <div className="mt-3 pt-3 border-t text-sm text-gray-500 dark:text-gray-400 dark:border-gray-700">
                        <span className="text-orange-500">*</span> {t("settings.requiredVars")}: {ext.requiredVariables.map((v: any) => v.name).join(", ")}
                        <span className="ml-2 text-gray-400 dark:text-gray-500">{t("settings.extensionVarsTip")}</span>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === "plugins" && (
            <div className="space-y-4">
              <h3 className="font-medium">{t("settings.featurePlugins")}</h3>
              <p className="text-sm text-gray-500 dark:text-gray-400">{t("settings.featurePluginsDesc")}</p>
              <div className="space-y-3">
                {featurePlugins.map((plugin) => (
                  <div key={plugin.id} className="bg-gray-50 rounded-xl border p-4 dark:bg-gray-900 dark:border-gray-700">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <div className="w-10 h-10 bg-blue-600 rounded-lg flex items-center justify-center text-white">
                          {plugin.plugin_type === "radar" && <i className="fa-solid fa-satellite-dish text-xl"></i>}
                          {plugin.plugin_type === "search" && <i className="fa-solid fa-magnifying-glass text-xl"></i>}
                          {plugin.plugin_type !== "radar" && plugin.plugin_type !== "search" && <i className="fa-solid fa-puzzle-piece text-xl"></i>}
                        </div>
                        <div>
                          <div className="flex items-center gap-2">
                            <span className="font-medium">{plugin.name}</span>
                            <span className="text-xs bg-gray-200 text-gray-600 px-2 py-0.5 rounded dark:bg-gray-700 dark:text-gray-300">
                              v{plugin.version || "1.0.0"}
                            </span>
                            <span className={`text-xs px-2 py-0.5 rounded ${
                              plugin.enabled
                                ? "bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300"
                                : "bg-gray-200 text-gray-500 dark:bg-gray-700 dark:text-gray-400"
                            }`}>
                              {plugin.enabled ? t("settings.enabled") : t("settings.disabled")}
                            </span>
                          </div>
                          <p className="text-sm text-gray-500 dark:text-gray-400">
                            {plugin.plugin_type} · {plugin.db_mode === "none" ? t("settings.dbModeNone") : t("settings.dbModeVault")}
                          </p>
                        </div>
                      </div>
                      <label className="relative inline-flex items-center cursor-pointer">
                        <input
                          type="checkbox"
                          checked={plugin.enabled}
                          onChange={(e) => {
                            invoke("set_plugin_enabled", { pluginId: plugin.id, enabled: e.target.checked });
                            setFeaturePlugins(featurePlugins.map((p) =>
                              p.id === plugin.id ? { ...p, enabled: e.target.checked } : p
                            ));
                          }}
                          className="sr-only peer"
                        />
                        <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600 dark:bg-gray-700 dark:after:border-gray-600"></div>
                      </label>
                    </div>
                  </div>
                ))}
                {featurePlugins.length === 0 && (
                  <p className="text-gray-500 dark:text-gray-400 text-center py-8">{t("settings.noPlugins")}</p>
                )}
              </div>
            </div>
          )}
        </div>

        <div className="p-6 border-t border-gray-200 flex items-center justify-between bg-gray-50 dark:border-gray-700 dark:bg-gray-900">
          <div>
            {saved && (
              <span className="text-sm text-green-600 flex items-center gap-1">
                <i className="fa-solid fa-check-circle"></i>{t("settings.settingsSaved")}
              </span>
            )}
          </div>
          <div className="flex gap-3">
            <button onClick={onClose} className="px-4 py-2 border rounded-lg hover:bg-white dark:border-gray-600 dark:hover:bg-gray-700 dark:text-gray-200">
              {t("common.cancel")}
            </button>
            <button
              onClick={handleSave}
              disabled={saving}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
            >
              {saving ? t("common.saving") : t("common.save")}
            </button>
          </div>
        </div>
      </div>

      {varModalOpen && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-[60]">
          <div className="bg-white rounded-xl shadow-xl w-full max-w-md dark:bg-gray-800">
            <div className="p-6 border-b dark:border-gray-700">
              <div className="flex items-center justify-between">
                <h3 className="text-lg font-medium">
                  {editingVarKey ? t("settings.editVariable") : t("settings.addVariable")}
                </h3>
                <button onClick={() => setVarModalOpen(false)} className="text-gray-400 hover:text-gray-600 dark:text-gray-500 dark:hover:text-gray-300">
                  <i className="fa-solid fa-xmark text-xl"></i>
                </button>
              </div>
            </div>
            <div className="p-6 space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2">{t("settings.variableKey")}</label>
                <input
                  type="text"
                  value={varForm.key}
                  onChange={(e) => setVarForm({ ...varForm, key: e.target.value })}
                  disabled={!!editingVarKey}
                  placeholder="github_token"
                  className="w-full px-3 py-2 border rounded-lg font-mono disabled:bg-gray-100 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200 dark:disabled:bg-gray-700"
                />
              </div>
              <div>
                <label className="block text-sm font-medium mb-2">{t("settings.variableValue")}</label>
                <div className="relative">
                  <input
                    type={varForm.isSecret && !showVarValue ? "password" : "text"}
                    value={varForm.value}
                    onChange={(e) => setVarForm({ ...varForm, value: e.target.value })}
                    placeholder={varForm.isSecret ? "••••••••" : t("settings.inputVariableValue")}
                    className="w-full px-3 py-2 border rounded-lg font-mono pr-10 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
                  />
                  {varForm.isSecret && (
                    <button
                      type="button"
                      onClick={() => setShowVarValue(!showVarValue)}
                      className="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 dark:text-gray-500 dark:hover:text-gray-300"
                      title={showVarValue ? t("settings.hide") : t("settings.show")}
                    >
                      <i className={`fa-solid ${showVarValue ? "fa-eye-slash" : "fa-eye"}`}></i>
                    </button>
                  )}
                </div>
              </div>
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={varForm.isSecret}
                  onChange={(e) => setVarForm({ ...varForm, isSecret: e.target.checked })}
                  className="rounded"
                />
                <span className="text-sm">{t("settings.secretTip")}</span>
              </label>
            </div>
            <div className="p-6 border-t flex justify-end gap-3 dark:border-gray-700">
              <button
                onClick={() => setVarModalOpen(false)}
                className="px-4 py-2 border rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 dark:border-gray-600 dark:text-gray-200"
              >
                {t("common.cancel")}
              </button>
              <button
                onClick={() => {
                  if (!varForm.key || !varForm.value) return;
                  invoke("set_system_variable", {
                    key: varForm.key,
                    value: varForm.value,
                    isSecret: varForm.isSecret,
                  }).then(() => {
                    invoke<SystemVariable[]>("get_system_variables").then(setVariables);
                    setVarModalOpen(false);
                  });
                }}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
              >
                {t("common.save")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
