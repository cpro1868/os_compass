import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";

function formatTime(value: string | null): string {
  if (!value) return "";
  const num = parseInt(value, 10);
  if (!isNaN(num) && num > 1e12) {
    return new Date(num).toLocaleString("zh-CN", { hour12: false });
  }
  if (/^\d{4}-\d{2}-\d{2}/.test(value)) {
    return value;
  }
  return value;
}

interface ProjectClone {
  id: number;
  project_id: number;
  cloned_path: string | null;
  cloned_at: string | null;
  proxy_enabled: boolean;
  proxy_protocol: string;
  proxy_host: string;
  proxy_port: number;
  proxy_username: string;
  proxy_password: string | null;
}

interface Project {
  id: number;
  name: string;
  url: string | null;
}

export function ClonePanel({ project }: { project: Project }) {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<ProjectClone | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [cloning, setCloning] = useState(false);
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState("");

  useEffect(() => {
    setLoading(true);
    setError("");

    const timer = setTimeout(() => {
      setLoading(false);
      if (!settings) {
        setError(t("clone.loadTimeout"));
      }
    }, 10000);

    invoke<ProjectClone>("get_clone_settings", { projectId: project.id })
      .then(data => {
        clearTimeout(timer);
        setSettings(data);
        setLoading(false);
      })
      .catch(e => {
        clearTimeout(timer);
        setError(String(e));
        setLoading(false);
      });

    return () => clearTimeout(timer);
  }, [project.id]);

  const showMsg = (text: string) => {
    setMsg(text);
    setTimeout(() => setMsg(""), 3000);
  };

  const handleSave = async () => {
    if (!settings) return;
    setSaving(true);
    try {
      await invoke("save_clone_settings", { clone: settings });
      showMsg(t("clone.saved"));
    } catch (e) {
      showMsg(t("clone.saveFailed") + ": " + String(e));
    }
    setSaving(false);
  };

  const normalizePath = (p: string | null) => p ? p.replace(/\//g, "\\") : p;

  const handleClone = async () => {
    if (!settings?.cloned_path) {
      showMsg(t("clone.setAndSaveFirst"));
      return;
    }

    const targetPath = `${settings.cloned_path}\\${project.name}`;
    if (await invoke<boolean>("path_exists", { path: targetPath })) {
      showMsg(`${t("clone.targetExists")}: ${targetPath}`);
      return;
    }

    setCloning(true);
    try {
      const result = await invoke<string>("execute_clone", {
        projectId: project.id,
        gitUrl: project.url || "",
        projectName: project.name,
      });
      showMsg(`${t("clone.cloneSuccess")}: ${result}`);
      const updated = await invoke<ProjectClone>("get_clone_settings", { projectId: project.id });
      setSettings(updated);
    } catch (e) {
      showMsg(t("clone.cloneFailed") + ": " + String(e));
    }
    setCloning(false);
  };

  const buildCommand = () => {
    if (!settings?.cloned_path) {
      return `git clone ${project.url || ""} ./${project.name}`;
    }
    const dir = normalizePath(settings.cloned_path);
    const fullPath = `${dir}\\${project.name}`;
    let cmd = `git clone ${project.url || ""} "${fullPath}"`;
    if (settings.proxy_enabled && settings.proxy_host && settings.proxy_port) {
      const auth = settings.proxy_username ? `${settings.proxy_username}:${settings.proxy_password || ""}@` : "";
      const proxy = `${settings.proxy_protocol}://${auth}${settings.proxy_host}:${settings.proxy_port}`;
      cmd = `set HTTP_PROXY=${proxy} && ${cmd}`;
    }
    return cmd;
  };

  const handleCopy = async () => {
    const cmd = buildCommand();
    await navigator.clipboard.writeText(cmd);
    showMsg(t("clone.copiedToClipboard"));
  };

  if (loading) {
    return (
      <div className="p-8 text-center">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 dark:border-blue-400 mx-auto mb-3" />
        <p className="text-gray-500 dark:text-gray-400">{t("common.loading")}</p>
      </div>
    );
  }

  if (error && !settings) {
    return (
      <div className="p-8 text-center">
        <p className="text-red-500 dark:text-red-400 mb-4">{error}</p>
        <button
          onClick={() => {
            setLoading(true);
            setError("");
            invoke<ProjectClone>("get_clone_settings", { projectId: project.id })
              .then(setSettings)
              .catch(e => setError(String(e)))
              .finally(() => setLoading(false));
          }}
          className="px-4 py-2 bg-blue-600 dark:bg-blue-500 text-white rounded"
        >
          {t("clone.retry")}
        </button>
      </div>
    );
  }

  if (!settings) {
    return <div className="p-8 text-center text-gray-500 dark:text-gray-400">{t("clone.cannotLoad")}</div>;
  }

  return (
    <div className="p-6 space-y-4">
      {msg && (
        <div className="bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-300 px-4 py-2 rounded-lg text-sm">{msg}</div>
      )}

      <div className="bg-white dark:bg-gray-800 rounded-lg border dark:border-gray-700 p-4">
        <div className="mb-3">
          <label className="block text-sm font-medium mb-1">{t("clone.targetPath")}</label>
          <div className="flex gap-2">
            <input
              type="text"
              value={settings.cloned_path || ""}
              onChange={e => setSettings({ ...settings, cloned_path: e.target.value || null })}
              placeholder={t("clone.cloneDirPlaceholder")}
              className="flex-1 px-3 py-2 border rounded text-sm dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
            />
            <button
              onClick={async () => {
                const dir = await open({ directory: true, title: t("clone.selectDir") });
                if (dir) setSettings({ ...settings, cloned_path: dir as string });
              }}
              className="px-3 py-2 border rounded text-sm dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
            >
              {t("clone.browse")}
            </button>
          </div>
        </div>

        <div className="mb-3">
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={settings.proxy_enabled}
              onChange={e => setSettings({ ...settings, proxy_enabled: e.target.checked })}
            />
            <span className="text-sm">{t("clone.enableProxy")}</span>
          </label>
        </div>

        {settings.proxy_enabled && (
          <div className="grid grid-cols-2 gap-3 mb-3 pl-6">
            <div>
              <label className="block text-xs mb-1">{t("clone.protocol")}</label>
              <select
                value={settings.proxy_protocol}
                onChange={e => setSettings({ ...settings, proxy_protocol: e.target.value })}
                className="w-full px-2 py-1.5 border rounded text-sm dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
              >
                <option value="http">HTTP</option>
                <option value="https">HTTPS</option>
                <option value="socks5">SOCKS5</option>
              </select>
            </div>
            <div>
              <label className="block text-xs mb-1">{t("clone.port")}</label>
              <input
                type="number"
                value={settings.proxy_port || ""}
                onChange={e => setSettings({ ...settings, proxy_port: parseInt(e.target.value) || 0 })}
                className="w-full px-2 py-1.5 border rounded text-sm dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
              />
            </div>
            <div className="col-span-2">
              <label className="block text-xs mb-1">{t("clone.address")}</label>
              <input
                type="text"
                value={settings.proxy_host}
                onChange={e => setSettings({ ...settings, proxy_host: e.target.value })}
                placeholder="127.0.0.1"
                className="w-full px-2 py-1.5 border rounded text-sm dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
              />
            </div>
            <div>
              <label className="block text-xs mb-1">{t("clone.username")}</label>
              <input
                type="text"
                value={settings.proxy_username}
                onChange={e => setSettings({ ...settings, proxy_username: e.target.value })}
                className="w-full px-2 py-1.5 border rounded text-sm dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
              />
            </div>
            <div>
              <label className="block text-xs mb-1">{t("clone.password")}</label>
              <input
                type="password"
                value={settings.proxy_password || ""}
                onChange={e => setSettings({ ...settings, proxy_password: e.target.value || null })}
                className="w-full px-2 py-1.5 border rounded text-sm dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200"
              />
            </div>
          </div>
        )}

        <button
          onClick={handleSave}
          disabled={saving}
          className="w-full py-2 bg-blue-600 dark:bg-blue-500 text-white rounded hover:bg-blue-700 dark:hover:bg-blue-600 disabled:opacity-50"
        >
          {saving ? t("common.saving") : t("clone.saveSettings")}
        </button>
      </div>

      <div className="bg-white dark:bg-gray-800 rounded-lg border dark:border-gray-700 p-4">
        <h3 className="font-medium mb-2">{t("clone.cloneCommand")}</h3>
        <pre className="bg-gray-900 text-gray-100 p-3 rounded text-sm overflow-x-auto mb-3 whitespace-pre-wrap">{buildCommand()}</pre>
        <div className="flex gap-2">
          <button
            onClick={handleCopy}
            className="flex-1 py-2 border dark:border-gray-600 rounded hover:bg-gray-50 dark:hover:bg-gray-700"
          >
            {t("clone.copyCommand")}
          </button>
          <button
            onClick={handleClone}
            disabled={cloning || !settings.cloned_path}
            className="flex-1 py-2 bg-green-600 dark:bg-green-500 text-white rounded hover:bg-green-700 dark:hover:bg-green-600 disabled:opacity-50"
          >
            {cloning ? t("clone.cloning") : t("clone.executeClone")}
          </button>
        </div>
      </div>

      {settings.cloned_at && (
        <div className="bg-green-50 dark:bg-green-950 border-green-200 dark:border-green-800 rounded-lg p-4">
          <div className="flex items-center gap-2 mb-2">
            <i className="fa-solid fa-check-circle text-green-600 dark:text-green-400"></i>
            <span className="font-medium text-green-800 dark:text-green-300">{t("clone.cloneLog")}</span>
          </div>
          <div className="text-sm space-y-1">
            <div className="flex justify-between">
              <span className="text-gray-500 dark:text-gray-400">{t("clone.status")}：</span>
              <span className="text-green-600 dark:text-green-400 font-medium">{t("clone.statusSuccess")}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-500 dark:text-gray-400">{t("clone.path")}：</span>
              <span className="text-gray-700 dark:text-gray-300">{normalizePath(settings.cloned_path)}\{project.name}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-500 dark:text-gray-400">{t("clone.time")}：</span>
              <span className="text-gray-600 dark:text-gray-400">{formatTime(settings.cloned_at)}</span>
            </div>
          </div>
        </div>
      )}
      {msg && !settings.cloned_at && (
        <div className="bg-red-50 dark:bg-red-950 border-red-200 dark:border-red-800 rounded-lg p-4">
          <div className="flex items-center gap-2 mb-2">
            <i className="fa-solid fa-circle-xmark text-red-500 dark:text-red-400"></i>
            <span className="font-medium text-red-800 dark:text-red-300">{t("clone.cloneLog")}</span>
          </div>
          <div className="text-sm space-y-1">
            <div className="flex justify-between">
              <span className="text-gray-500 dark:text-gray-400">{t("clone.status")}：</span>
              <span className="text-red-600 dark:text-red-400 font-medium">{t("clone.statusFailed")}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-500 dark:text-gray-400">{t("clone.path")}：</span>
              <span className="text-gray-700 dark:text-gray-300">{normalizePath(settings.cloned_path)}\{project.name}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-gray-500 dark:text-gray-400">{t("clone.time")}：</span>
              <span className="text-gray-600 dark:text-gray-400">{new Date().toLocaleString('zh-CN', { hour12: false })}</span>
            </div>
            <div className="flex justify-between mt-2 pt-2 border-t border-red-200 dark:border-red-800">
              <span className="text-gray-500 dark:text-gray-400">{t("clone.reason")}：</span>
              <span className="text-red-600 dark:text-red-400 text-right max-w-xs">{msg.replace(t("clone.cloneFailedPrefix"), "")}</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
