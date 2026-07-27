import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import "./locales/i18n";
import { useAppStore } from "./stores/appStore";
import { useTheme } from "./hooks/useTheme";
import { getSettings } from "./api";
import { type Project, type LifecycleStatus } from "./types";
import { restoreProject, getArchivedProjects, batchArchiveProjects, batchDeleteProjects, batchRestoreProjects, batchMoveCategory } from "./api";
import { Sidebar } from "./components/Sidebar";
import { SettingsDialog } from "./components/SettingsDialog";
import { VaultDialog } from "./components/VaultDialog";
import { ProjectDetailDialog } from "./components/ProjectDetailDialog";
import { KanbanView } from "./components/KanbanView";
import { ListView } from "./components/ListView";
import { StatsView } from "./components/StatsView";
import { ImportModal } from "./components/ImportModal";
import { ArchiveView } from "./components/ArchiveView";
import { ErrorState } from "./components/ErrorState";
import { RadarInbox } from "./components/RadarInbox";
import { SearchView } from "./components/SearchView";
import { OrganizationManager } from "./components/OrganizationManager";
import { ToastContainer } from "./stores/toastStore";
import { NavigationDialog } from "./components/NavigationDialog";
import { InitWizard } from "./components/InitWizard";
import { HomePage } from "./components/HomePage";
import { getCurrentVault } from "./api";

type ViewMode = "home" | "kanban" | "list" | "archive" | "category" | "tag" | "stats" | "radar" | "search" | "organization";

function App() {
  const { t } = useTranslation();
  const { projects, loading, error, fetchProjects, fetchCategories, fetchTags, updateStatus } = useAppStore();
  const [showImport, setShowImport] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showVault, setShowVault] = useState(false);
  const [showNavigation, setShowNavigation] = useState(false);
  const [showInitWizard, setShowInitWizard] = useState(false);
  const [isManualInitWizard, setIsManualInitWizard] = useState(false);
  const [selectedProject, setSelectedProject] = useState<Project | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>("home");
  const [archivedProjects, setArchivedProjects] = useState<Project[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [theme, setTheme] = useState<string>("dark");

  useEffect(() => {
    getSettings().then((s) => setTheme(s.theme)).catch(() => {});
  }, []);

  useTheme(theme);

  const handleThemeChange = (newTheme: string) => {
    setTheme(newTheme);
  };

  // 检测是否需要显示初始化向导
  useEffect(() => {
    const checkVault = async () => {
      try {
        const vault = await getCurrentVault();
        if (!vault) {
          setShowInitWizard(true);
        }
      } catch {
        setShowInitWizard(true);
      }
    };
    checkVault();
  }, []);

  useEffect(() => {
    fetchProjects();
    fetchCategories();
    fetchTags();
  }, []);

  // 监听仓库切换事件，重新加载所有数据
  useEffect(() => {
    const handler = () => {
      console.log("[App] vault-changed event received, reloading data");
      fetchProjects();
      fetchCategories();
      fetchTags();
      setArchivedProjects([]);
      setSelectedIds(new Set());
      setSelectedProject(null);
    };
    window.addEventListener("vault-changed", handler);
    return () => window.removeEventListener("vault-changed", handler);
  }, [fetchProjects, fetchCategories, fetchTags]);

  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail?.projectId) {
        const project = projects.find((p) => p.id === detail.projectId);
        if (project) setSelectedProject(project);
      }
    };
    window.addEventListener("openProjectDetail", handler);
    return () => window.removeEventListener("openProjectDetail", handler);
  }, [projects]);

  useEffect(() => {
    const handler = () => setShowImport(true);
    window.addEventListener("openImport", handler);
    return () => window.removeEventListener("openImport", handler);
  }, []);

  // 监听导航事件
  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail?.view) {
        setViewMode(detail.view as any);
      }
      if (detail?.openSettings) {
        setShowSettings(true);
      }
      if (detail?.openVault) {
        setShowVault(true);
      }
      if (detail?.openInitWizard) {
        setShowInitWizard(true);
      }
      if (detail?.openManualAdd) {
        // TODO: 打开手动录入
      }
    };
    window.addEventListener("navigate", handler);
    return () => window.removeEventListener("navigate", handler);
  }, []);

  useEffect(() => {
    setSelectedIds(new Set());
  }, [viewMode]);

  const handleStatusChange = async (projectId: number, newStatus: LifecycleStatus) => {
    await updateStatus(projectId, newStatus);
  };

  const handleRestore = async (id: number) => {
    await restoreProject(id);
    setArchivedProjects(await getArchivedProjects());
    fetchProjects();
  };

  const handlePermanentDelete = async (id: number) => {
    await batchDeleteProjects([id]);
    setArchivedProjects(await getArchivedProjects());
    fetchProjects();
  };

  const handleBatchRestoreArchive = async (ids: number[]) => {
    await batchRestoreProjects(ids);
    setArchivedProjects(await getArchivedProjects());
    fetchProjects();
  };

  const handleBatchPermanentDelete = async (ids: number[]) => {
    await batchDeleteProjects(ids);
    setArchivedProjects(await getArchivedProjects());
    fetchProjects();
  };

  const openArchive = async () => {
    setViewMode("archive");
    setArchivedProjects(await getArchivedProjects());
  };

  const openCategory = () => {
    setViewMode("organization");
  };

  const handleBatchArchive = async () => {
    if (selectedIds.size === 0) return;
    await batchArchiveProjects(Array.from(selectedIds));
    setSelectedIds(new Set());
    fetchProjects();
  };

  const handleBatchRestore = async () => {
    if (selectedIds.size === 0) return;
    await batchRestoreProjects(Array.from(selectedIds));
    setSelectedIds(new Set());
    setArchivedProjects(await getArchivedProjects());
    fetchProjects();
  };

  const handleBatchDelete = async () => {
    if (selectedIds.size === 0) return;
    if (confirm(`确定永久删除 ${selectedIds.size} 个项目？`)) {
      await batchDeleteProjects(Array.from(selectedIds));
      setSelectedIds(new Set());
      setArchivedProjects(await getArchivedProjects());
      fetchProjects();
    }
  };

  const handleImportSuccess = () => {
    setShowImport(false);
    fetchProjects();
  };

  const viewTitle = viewMode === "kanban" ? t("nav.kanban") : viewMode === "list" ? t("nav.list") : t("archive.title");

  return (
    <>
      {showInitWizard && (
        <InitWizard onComplete={() => {
          setShowInitWizard(false);
          setIsManualInitWizard(false);
          fetchProjects();
          fetchCategories();
          fetchTags();
        }} isManual={isManualInitWizard} />
      )}
      <div className="flex h-screen bg-gray-50 dark:bg-gray-900">
      <Sidebar
        onSettings={() => setShowSettings(true)}
        onOpenVault={() => setShowVault(true)}
        onOpenCategory={openCategory}
        onOpenNavigation={() => setShowNavigation(true)}
        onOpenInitWizard={() => { setIsManualInitWizard(true); setShowInitWizard(true); }}
        currentView={viewMode}
        onViewChange={setViewMode}
      />

      <main className="flex-1 flex flex-col overflow-hidden">
        {viewMode === "home" ? (
          <HomePage />
        ) : (
          <div className="flex flex-col h-full">
            {viewMode !== "radar" && viewMode !== "search" && (
              <header className="bg-white dark:bg-gray-800 shadow-sm border-b border-gray-200 dark:border-gray-700 px-6 py-4 flex items-center justify-between flex-shrink-0">
                <div className="flex items-center gap-4">
                  <h1 className="text-xl font-bold text-gray-900 dark:text-gray-100">{viewTitle}</h1>
                  <div className="flex gap-1">
                    <button
                      onClick={() => setViewMode("kanban")}
                      className={`px-3 py-1 text-sm rounded ${
                        viewMode === "kanban"
                          ? "bg-indigo-100 text-indigo-700 dark:bg-indigo-900 dark:text-indigo-300"
                          : "text-gray-600 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-700"
                      }`}
                    >
                      {t("nav.kanban")}
                    </button>
                    <button
                      onClick={() => setViewMode("list")}
                      className={`px-3 py-1 text-sm rounded ${
                        viewMode === "list"
                          ? "bg-indigo-100 text-indigo-700 dark:bg-indigo-900 dark:text-indigo-300"
                          : "text-gray-600 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-700"
                      }`}
                    >
                      {t("nav.list")}
                    </button>
                    <button
                      onClick={openArchive}
                      className={`px-3 py-1 text-sm rounded ${
                        viewMode === "archive"
                          ? "bg-indigo-100 text-indigo-700 dark:bg-indigo-900 dark:text-indigo-300"
                          : "text-gray-600 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-700"
                      }`}
                    >
                      {t("nav.archive")} ({archivedProjects.length || ""})
                    </button>
                    <button
                      onClick={() => setViewMode("stats")}
                      className={`px-3 py-1 text-sm rounded ${
                        viewMode === "stats"
                          ? "bg-indigo-100 text-indigo-700 dark:bg-indigo-900 dark:text-indigo-300"
                          : "text-gray-600 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-700"
                      }`}
                    >
                      <i className="fa-solid fa-chart-pie mr-1"></i>统计
                    </button>
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  {selectedIds.size > 0 && (
                    <div className="flex items-center gap-2 px-3 py-1 bg-indigo-50 rounded-lg">
                      <span className="text-sm text-indigo-700">
                        {t("list.selected", { count: selectedIds.size })}
                      </span>
                      {viewMode === "archive" ? (
                        <>
                          <button
                            onClick={handleBatchRestore}
                            className="px-2 py-1 text-xs bg-green-600 text-white rounded hover:bg-green-700"
                          >
                            {t("actions.batchRestore")}
                          </button>
                          <button
                            onClick={handleBatchDelete}
                            className="px-2 py-1 text-xs bg-red-600 text-white rounded hover:bg-red-700"
                          >
                            {t("actions.batchDelete")}
                          </button>
                        </>
                      ) : (
                        <>
                          <button
                            onClick={handleBatchArchive}
                            className="px-2 py-1 text-xs bg-orange-600 text-white rounded hover:bg-orange-700"
                          >
                            {t("actions.batchArchive")}
                          </button>
                          <button
                            onClick={handleBatchDelete}
                            className="px-2 py-1 text-xs bg-red-600 text-white rounded hover:bg-red-700"
                          >
                            {t("actions.batchDelete")}
                          </button>
                        </>
                      )}
                    </div>
                  )}
                  {viewMode !== "archive" && viewMode !== "category" && (
                    <button
                      onClick={() => setShowImport(true)}
                      className="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700"
                    >
                      + {t("actions.import")}
                    </button>
                  )}
                </div>
              </header>
            )}

            <div className="flex-1 overflow-hidden">
              {error && (
                <ErrorState
                  title="数据加载失败"
                  message={error}
                  onRetry={() => fetchProjects()}
                />
              )}

              {loading ? (
                <div className="flex items-center justify-center h-full text-gray-500">
                  <div className="text-center">
                    <div className="inline-block animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600 mb-2"></div>
                    <p>{t("common.loading")}</p>
                  </div>
                </div>
              ) : viewMode === "kanban" ? (
                <KanbanView
                  projects={projects}
                  onProjectClick={setSelectedProject}
                  onStatusChange={handleStatusChange}
                  selectedIds={selectedIds}
                  onSelectionChange={setSelectedIds}
                  onBatchArchive={async (ids) => {
                    await batchArchiveProjects(ids);
                    fetchProjects();
                  }}
                  onBatchMoveCategory={async (ids, categoryId) => {
                    await batchMoveCategory(ids, categoryId);
                    fetchProjects();
                  }}
                  onBatchExport={(projects) => {
                    const exportData = projects.map((p) => ({
                      name: p.name,
                      url: p.url,
                      source: p.source,
                      description: p.description,
                      languages: p.languages,
                      stars: p.stars,
                      forks: p.forks,
                      license: p.license,
                      lifecycle_status: p.lifecycle_status,
                      health_score: p.health_score,
                      ai_summary: p.ai_summary,
                      created_at: p.created_at,
                    }));
                    const json = JSON.stringify(exportData, null, 2);
                    const blob = new Blob([json], { type: "application/json" });
                    const url = URL.createObjectURL(blob);
                    const a = document.createElement("a");
                    a.href = url;
                    a.download = `os-compass-export-${new Date().toISOString().slice(0, 10)}.json`;
                    a.click();
                    URL.revokeObjectURL(url);
                  }}
                />
              ) : viewMode === "list" ? (
                <ListView
                  projects={projects}
                  onProjectClick={setSelectedProject}
                  selectedIds={selectedIds}
                  onSelectionChange={setSelectedIds}
                  onBatchArchive={async (ids) => {
                    await batchArchiveProjects(ids);
                    fetchProjects();
                  }}
                />
              ) : viewMode === "category" ? (
                <OrganizationManager />
              ) : viewMode === "tag" ? (
                <OrganizationManager />
              ) : viewMode === "stats" ? (
                <StatsView onBack={() => setViewMode("kanban")} />
              ) : viewMode === "radar" ? (
                <RadarInbox />
              ) : viewMode === "search" ? (
                <SearchView />
              ) : viewMode === "organization" ? (
                <OrganizationManager />
              ) : (
                <ArchiveView
                  onBack={() => setViewMode("list")}
                  projects={archivedProjects}
                  onRestore={handleRestore}
                  onPermanentDelete={handlePermanentDelete}
                  onBatchRestore={handleBatchRestoreArchive}
                  onBatchPermanentDelete={handleBatchPermanentDelete}
                />
              )}
            </div>
          </div>
        )}
      </main>

      {showImport && (
        <ImportModal
          onClose={() => setShowImport(false)}
          onSuccess={handleImportSuccess}
        />
      )}

      <SettingsDialog open={showSettings} onClose={() => setShowSettings(false)} onThemeChange={handleThemeChange} />
      <VaultDialog open={showVault} onClose={() => setShowVault(false)} />
      <NavigationDialog open={showNavigation} onClose={() => setShowNavigation(false)} />

      {selectedProject && (
        <ProjectDetailDialog
          project={selectedProject}
          open={!!selectedProject}
          onClose={() => setSelectedProject(null)}
          onUpdate={() => {
            fetchProjects();
          }}
          onDeleted={() => {
            setSelectedProject(null);
            fetchProjects();
          }}
        />
      )}
      <ToastContainer />
    </div>
    </>
  );
}

export default App;
