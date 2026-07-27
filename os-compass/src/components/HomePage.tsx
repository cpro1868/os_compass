import { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getRecentProjects } from "../api";
import type { Project } from "../types";
import { ImportModal } from "./ImportModal";

interface ProjectPreview {
  name: string;
  url: string;
  platform: string;
  icon: string;
  stars?: string;
  language?: string;
}

const PLATFORMS = [
  { pattern: /github\.com/i, name: "GitHub", icon: "fa-brands fa-github" },
  { pattern: /gitee\.com/i, name: "Gitee", icon: "fa-solid fa-code-branch" },
  { pattern: /gitlab\.com/i, name: "GitLab", icon: "fa-brands fa-gitlab" },
  { pattern: /npmjs\.com/i, name: "NPM", icon: "fa-brands fa-npm" },
  { pattern: /pypi\.org/i, name: "PyPI", icon: "fa-brands fa-python" },
  { pattern: /crates\.io/i, name: "Crates.io", icon: "fa-solid fa-cube" },
];

export function HomePage() {
  const { t } = useTranslation();
  const [url, setUrl] = useState("");
  const [preview, setPreview] = useState<ProjectPreview | null>(null);
  const [recentProjects, setRecentProjects] = useState<Project[]>([]);
  const [showImport, setShowImport] = useState(false);
  const [loading, setLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    loadRecentProjects();
    inputRef.current?.focus();
  }, []);

  const loadRecentProjects = async () => {
    try {
      const projects = await getRecentProjects(5);
      setRecentProjects(projects);
    } catch {
      console.error("Failed to load recent projects");
    }
  };

  const detectPlatform = (url: string) => {
    for (const p of PLATFORMS) {
      if (p.pattern.test(url)) {
        return p;
      }
    }
    return null;
  };

  const fetchPreview = async (url: string) => {
    const platform = detectPlatform(url);
    if (!platform) {
      setPreview(null);
      return;
    }

    setLoading(true);
    try {
      const info = await invoke<{ name?: string; stars?: string; language?: string }>("get_project_preview", { url });
      setPreview({
        name: info.name || extractProjectName(url),
        url,
        platform: platform.name,
        icon: platform.icon,
        stars: info.stars,
        language: info.language,
      });
    } catch {
      setPreview({
        name: extractProjectName(url),
        url,
        platform: platform.name,
        icon: platform.icon,
      });
    } finally {
      setLoading(false);
    }
  };

  const extractProjectName = (url: string): string => {
    const match = url.match(/\/([^\/]+)\/?$/);
    return match ? match[1] : url;
  };

  const handleUrlChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setUrl(value);
    if (value.length > 10) {
      fetchPreview(value);
    } else {
      setPreview(null);
    }
  };

  const handleImport = () => {
    if (preview) {
      setUrl(preview.url);
      setShowImport(true);
    }
  };

  const handleImportSuccess = () => {
    setShowImport(false);
    setUrl("");
    setPreview(null);
    loadRecentProjects();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && preview) {
      handleImport();
    }
  };

  const handleProjectClick = async (project: Project) => {
    const window = getCurrentWindow();
    window.emit("openProjectDetail", { projectId: project.id });
  };

  const getPlatformIcon = (url: string) => {
    const platform = detectPlatform(url);
    return platform?.icon || "fa-solid fa-link";
  };

  return (
    <div className="flex-1 flex flex-col items-center justify-center min-h-full px-4 py-8 bg-gradient-to-br from-slate-50 to-slate-100 dark:from-gray-900 dark:to-slate-900">
      <div className="w-full max-w-2xl mb-8">
        <div className="text-center mb-8">
          <div className="inline-flex items-center justify-center w-16 h-16 bg-gradient-to-br from-blue-500 to-indigo-600 rounded-2xl mb-4 shadow-lg">
            <i className="fa-solid fa-compass text-3xl text-white"></i>
          </div>
          <h1 className="text-3xl font-bold text-gray-900 dark:text-gray-100 mb-2">开源罗盘</h1>
          <p className="text-gray-500 dark:text-gray-400">AI 开源资产智能管家</p>
        </div>

        <div className="relative">
          <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-xl border border-gray-200 dark:border-gray-700 p-2">
            <div className="flex items-center gap-4">
              <div className="pl-4">
                <i className="fa-solid fa-link text-gray-400 text-xl"></i>
              </div>
              <input
                ref={inputRef}
                type="text"
                value={url}
                onChange={handleUrlChange}
                onKeyDown={handleKeyDown}
                placeholder={t("home.placeholder") || "粘贴开源项目链接..."}
                className="flex-1 bg-transparent text-lg py-4 focus:outline-none placeholder-gray-400 dark:placeholder-gray-500 text-gray-900 dark:text-gray-100"
              />
              <button
                onClick={handleImport}
                disabled={!preview || loading}
                className="px-6 py-3 bg-blue-600 hover:bg-blue-500 disabled:bg-gray-300 disabled:cursor-not-allowed rounded-xl font-medium transition flex items-center gap-2 text-white"
              >
                {loading ? (
                  <i className="fa-solid fa-spinner fa-spin"></i>
                ) : (
                  <>
                    <i className="fa-solid fa-arrow-right"></i>
                    <span>{t("home.import") || "导入"}</span>
                  </>
                )}
              </button>
            </div>
          </div>

          {preview && (
            <div className="mt-4 bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700 shadow-lg">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="w-12 h-12 bg-gray-100 dark:bg-gray-700 rounded-xl flex items-center justify-center">
                    <i className={`${preview.icon} text-2xl text-gray-600 dark:text-gray-300`}></i>
                  </div>
                  <div>
                    <div className="font-semibold text-gray-900 dark:text-gray-100">{preview.name}</div>
                    <div className="text-sm text-gray-500 dark:text-gray-400">{preview.platform}</div>
                  </div>
                </div>
                <div className="flex items-center gap-4 text-sm text-gray-500 dark:text-gray-400">
                  {preview.stars && (
                    <span className="flex items-center gap-1">
                      <i className="fa-solid fa-star text-yellow-500"></i>
                      {preview.stars}
                    </span>
                  )}
                  {preview.language && (
                    <span className="flex items-center gap-1">
                      <i className="fa-solid fa-code text-green-500"></i>
                      {preview.language}
                    </span>
                  )}
                </div>
              </div>
            </div>
          )}
        </div>

        <div className="mt-6 flex items-center justify-center gap-6 text-gray-400 dark:text-gray-500 text-sm">
          <span className="flex items-center gap-2">
            <i className="fa-brands fa-github"></i> GitHub
          </span>
          <span className="flex items-center gap-2">
            <i className="fa-solid fa-code-branch"></i> Gitee
          </span>
          <span className="flex items-center gap-2">
            <i className="fa-brands fa-npm"></i> NPM
          </span>
          <span className="flex items-center gap-2">
            <i className="fa-brands fa-python"></i> PyPI
          </span>
        </div>
      </div>

      {recentProjects.length > 0 && (
        <div className="w-full max-w-4xl">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-medium text-gray-700 dark:text-gray-300">{t("home.recentProjects") || "最近项目"}</h2>
          </div>

          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-4">
            {recentProjects.map((project) => (
              <div
                key={project.id}
                onClick={() => handleProjectClick(project)}
                className="bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700 hover:border-blue-500 dark:hover:border-blue-500 cursor-pointer transition group"
              >
                <div className="flex items-start justify-between mb-3">
                  <div className="w-10 h-10 bg-blue-100 dark:bg-blue-900/30 rounded-lg flex items-center justify-center">
                    <i className={`${getPlatformIcon(project.url || "")} text-blue-500`}></i>
                  </div>
                  {project.lifecycle_status && (
                    <span className="text-xs px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded text-gray-500 dark:text-gray-400">
                      {project.lifecycle_status}
                    </span>
                  )}
                </div>
                <h3 className="font-medium text-gray-900 dark:text-gray-100 group-hover:text-blue-500 transition truncate">
                  {project.name}
                </h3>
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-1 truncate">
                  {project.description || project.url}
                </p>
              </div>
            ))}
          </div>
        </div>
      )}

      {showImport && (
        <ImportModal
          onClose={() => setShowImport(false)}
          onSuccess={handleImportSuccess}
        />
      )}
    </div>
  );
}
