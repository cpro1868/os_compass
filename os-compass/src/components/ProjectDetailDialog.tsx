import { useEffect, useState, useCallback } from "react";
import type { Project, Category } from "../types";
import { parseLanguages } from "../types";
import { invoke } from "@tauri-apps/api/core";
import { refreshProjectReadme } from "../api";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { MarkdownRenderer } from "./MarkdownRenderer";
import { saveRunbook, tagApi, type Tag, saveReadmeTranslation, clearTranslations, deleteProject, testLlmDirect } from "../api";
import { getCategories } from "../api";
import { noteApi } from "../api/note";
import { ClonePanel } from "./ClonePanel";
import { ReleasesPanel } from "./ReleasesPanel";
import { useTranslation } from "react-i18next";

interface ProjectDetailDialogProps {
  project: Project;
  open: boolean;
  onClose: () => void;
  onUpdate: () => void;
  onDeleted?: () => void;
}

interface AiResult {
  summary: string | null;
  useCases: string | null;
  risks: string | null;
  dependencies: string | null;
  health_score: number | null;
  health_rating: string | null;
  error: string | null;
}

const STATUS_CONFIG: Record<string, { emoji: string; label: string }> = {
  TO_EXPLORE: { emoji: "💡", label: "待探索" },
  DIVING: { emoji: "🔬", label: "深度研究中" },
  IN_USE: { emoji: "✅", label: "已落地/在用" },
  ABANDONED: { emoji: "🗑️", label: "弃用/避坑" },
};

export function ProjectDetailDialog({ project: initialProject, open, onClose, onUpdate, onDeleted }: ProjectDetailDialogProps) {
  const { t } = useTranslation();
  const [project, setProject] = useState<Project>(initialProject);
  useEffect(() => { setProject(initialProject); }, [initialProject]);
  const [tab, setTab] = useState<"overview" | "ai" | "runbook" | "readme" | "clone" | "releases" | "notes" | "user">("overview");
  const [analyzing, setAnalyzing] = useState(false);
  const [aiResult, setAiResult] = useState<AiResult | null>(null);
  const [runbookContent, setRunbookContent] = useState<string | null>(null);
  const [isEditingRunbook, setIsEditingRunbook] = useState(false);
  const [editedRunbook, setEditedRunbook] = useState("");
  const [readmeLang, setReadmeLang] = useState<"original" | "translated">("original");
  const [translatedSummary, setTranslatedSummary] = useState<string | null>(null);
  const [translatedUseCases, setTranslatedUseCases] = useState<string | null>(null);
  const [translatedRisks, setTranslatedRisks] = useState<string | null>(null);
  const [translatedDeps, setTranslatedDeps] = useState<string | null>(null);
  const [translatedDescription, setTranslatedDescription] = useState<string | null>(null);
  const [isTranslating, setIsTranslating] = useState(false);
  const [isOverviewTranslated, setIsOverviewTranslated] = useState(false);
  const [translatingFields, setTranslatingFields] = useState<Set<string>>(new Set());
  const [backgroundTasks, setBackgroundTasks] = useState<{ taskType: string; status: string }[]>([]);
  const [projectTags, setProjectTags] = useState<Tag[]>([]);
  const [allTags, setAllTags] = useState<Tag[]>([]);
  const [showTagSelector, setShowTagSelector] = useState(false);
  const [isGeneratingTags, setIsGeneratingTags] = useState(false);
  const [readmeTranslation, setReadmeTranslation] = useState<string | null>(null);
  const [isTranslatingReadme, setIsTranslatingReadme] = useState(false);
  const [readmeVariants, setReadmeVariants] = useState<{id: number; project_id: number; file_name: string; language: string; content: string}[]>([]);
  const [selectedVariant, setSelectedVariant] = useState<string | null>(null);
  const [projectNotes, setProjectNotes] = useState<{id: number; content: string; created_at: string}[]>([]);
  const [noteInput, setNoteInput] = useState("");
  const [categories, setCategories] = useState<Category[]>([]);
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState("");
  const [editCategoryId, setEditCategoryId] = useState<number | null>(null);
  const [saving, setSaving] = useState(false);
  const [isClassifying, setIsClassifying] = useState(false);
  const [toasts, setToasts] = useState<{id: number; message: string; type: string}[]>([]);
  const [toastId, setToastId] = useState(0);
  const [localPathExists, setLocalPathExists] = useState<boolean | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  const showToast = (message: string, type: "success" | "error" | "info" = "info") => {
    const id = toastId + 1;
    setToastId(id);
    setToasts(prev => [...prev, { id, message, type }]);
    setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id));
    }, 3000);
  };

  useEffect(() => {
    if (open && project.id) {
      setAiResult(null);
      setRunbookContent(null);
      setTab("overview");
      setReadmeLang("original");
      setTranslatedSummary(null);
      setTranslatedUseCases(null);
      setTranslatedRisks(null);
      setTranslatedDeps(null);
      setTranslatedDescription(null);
      setIsOverviewTranslated(false);
      setReadmeTranslation(null);
      setProjectNotes([]);
      setNoteInput("");

      invoke<any>("get_project", { id: project.id })
        .then((freshProject) => {
          if (freshProject) {
            if (freshProject.ai_summary || freshProject.ai_use_cases || freshProject.ai_risks || freshProject.ai_dependencies) {
            const result: AiResult = {
              summary: freshProject.ai_summary || null,
              useCases: freshProject.ai_use_cases || null,
              risks: freshProject.ai_risks || null,
              dependencies: freshProject.ai_dependencies || null,
              health_score: freshProject.health_score || null,
              health_rating: freshProject.health_score ? `${Math.round(freshProject.health_score)}${t("detail.points")}` : null,
              error: null,
            };
            setAiResult(result);
            }

            if (freshProject.runbook) {
              setRunbookContent(freshProject.runbook);
            }

            // 翻译内容现在从 translations 表加载（见下方 invoke 调用）

            if (freshProject.readme_translation) setReadmeTranslation(freshProject.readme_translation);
          }
        })
        .catch(console.error);

        loadTags();
        noteApi.getByProject(project.id).then(setProjectNotes).catch(() => {});
        getCategories().then(setCategories).catch(() => {});

        invoke<any[]>("get_translations_cmd", { projectId: project.id, language: "zh-CN" })
          .then((translations) => {
            const trans: any[] = Array.isArray(translations) ? translations : [];
            if (trans.length > 0) {
              trans.forEach((t: any) => {
                switch (t.field_name) {
                  case "summary":
                    setTranslatedSummary(t.content);
                    break;
                  case "use_cases":
                    setTranslatedUseCases(t.content);
                    break;
                  case "risks":
                    setTranslatedRisks(t.content);
                    break;
                  case "dependencies":
                    setTranslatedDeps(t.content);
                    break;
                  case "description":
                    setTranslatedDescription(t.content);
                    break;
                }
              });
              setIsOverviewTranslated(true);
            }
          })
          .catch(() => {});

        invoke<any[]>("get_readme_variants_cmd", { projectId: project.id })
          .then((variants) => {
            setReadmeVariants(variants || []);
            if (variants && variants.length > 0) {
              setSelectedVariant(variants[0].file_name);
            } else {
              setSelectedVariant(null);
            }
          })
          .catch(() => setReadmeVariants([]));

      const lp = (initialProject as any).local_path;
      if (lp) {
        invoke<boolean>("path_exists", { path: lp })
          .then(setLocalPathExists)
          .catch(() => setLocalPathExists(false));
      } else {
        setLocalPathExists(null);
      }
    }
  }, [open, project.id]);

  useEffect(() => {
    if (!open || !project.id) return;
    let unlisten: UnlistenFn | null = null;
    listen<{ project_id: number; task_type: string; status: string; error: string | null }>("import:task_progress", (event) => {
      const { project_id, task_type, status } = event.payload;
      if (project_id !== project.id) return;

      if (task_type === "all" && status === "done") {
        setBackgroundTasks([]);
        invoke<any>("get_project", { id: project.id }).then((freshProject) => {
          if (freshProject) {
            if (freshProject.ai_summary || freshProject.ai_use_cases || freshProject.ai_risks || freshProject.ai_dependencies) {
              setAiResult({
                summary: freshProject.ai_summary || null,
                useCases: freshProject.ai_use_cases || null,
                risks: freshProject.ai_risks || null,
                dependencies: freshProject.ai_dependencies || null,
                health_score: freshProject.health_score || null,
                health_rating: freshProject.health_score ? `${Math.round(freshProject.health_score)}${t("detail.points")}` : null,
                error: null,
              });
            }
            if (freshProject.readme_translation) setReadmeTranslation(freshProject.readme_translation);
            if (freshProject.runbook) setRunbookContent(freshProject.runbook);
          }
        }).catch(() => {});
        invoke<any[]>("get_translations_cmd", { projectId: project.id, language: "zh-CN" }).then(async (translations) => {
          const trans: any[] = Array.isArray(translations) ? translations : [];
          const translatedFields = new Set(trans.map((t: any) => t.field_name));
          trans.forEach((t: any) => {
            switch (t.field_name) {
              case "summary": setTranslatedSummary(t.content); break;
              case "use_cases": setTranslatedUseCases(t.content); break;
              case "risks": setTranslatedRisks(t.content); break;
              case "dependencies": setTranslatedDeps(t.content); break;
              case "description": setTranslatedDescription(t.content); break;
            }
          });
          setIsOverviewTranslated(true);

          const missing: { field: string; text: string; setter: (v: string | null) => void }[] = [];
          if (aiResult?.summary && !translatedFields.has("summary")) missing.push({ field: "summary", text: aiResult.summary, setter: setTranslatedSummary });
          if (aiResult?.useCases && !translatedFields.has("use_cases")) missing.push({ field: "use_cases", text: aiResult.useCases, setter: setTranslatedUseCases });
          if (aiResult?.risks && !translatedFields.has("risks")) missing.push({ field: "risks", text: aiResult.risks, setter: setTranslatedRisks });
          if (aiResult?.dependencies && !translatedFields.has("dependencies")) missing.push({ field: "dependencies", text: aiResult.dependencies, setter: setTranslatedDeps });
          if (project.description && !translatedFields.has("description")) missing.push({ field: "description", text: project.description, setter: setTranslatedDescription });

          if (missing.length > 0) {
            console.log("[翻译补译] 缺失字段:", missing.map(m => m.field));
            for (const item of missing) {
              setTranslatingFields(prev => new Set(prev).add(item.field));
              try {
                const result = await invoke<string>("translate_with_llm", { text: item.text, targetLang: "zh-CN" });
                if (result && result.trim()) {
                  item.setter(result);
                  await invoke("save_translation_cmd", { projectId: project.id, fieldName: item.field, language: "zh-CN", content: result }).catch(() => {});
                }
              } catch (e) {
                console.error("[翻译补译] 失败:", item.field, e);
              } finally {
                setTranslatingFields(prev => {
                  const next = new Set(prev);
                  next.delete(item.field);
                  return next;
                });
              }
            }
          }
        }).catch(() => {});
        return;
      }

      setBackgroundTasks((prev) => {
        const filtered = prev.filter((t) => t.taskType !== task_type);
        if (status === "running" || status === "pending") {
          return [...filtered, { taskType: task_type, status }];
        }
        return filtered;
      });

      if (status === "done" && task_type !== "all") {
        invoke<any>("get_project", { id: project.id }).then((freshProject) => {
          if (freshProject) {
            if (freshProject.ai_summary || freshProject.ai_use_cases || freshProject.ai_risks || freshProject.ai_dependencies) {
              setAiResult({
                summary: freshProject.ai_summary || null,
                useCases: freshProject.ai_use_cases || null,
                risks: freshProject.ai_risks || null,
                dependencies: freshProject.ai_dependencies || null,
                health_score: freshProject.health_score || null,
                health_rating: freshProject.health_score ? `${Math.round(freshProject.health_score)}${t("detail.points")}` : null,
                error: null,
              });
            }
            if (freshProject.readme_translation) setReadmeTranslation(freshProject.readme_translation);
            if (freshProject.runbook) setRunbookContent(freshProject.runbook);
          }
        }).catch(() => {});
        invoke<any[]>("get_translations_cmd", { projectId: project.id, language: "zh-CN" }).then((translations) => {
          const trans: any[] = Array.isArray(translations) ? translations : [];
          if (trans.length > 0) {
            trans.forEach((t: any) => {
              switch (t.field_name) {
                case "summary": setTranslatedSummary(t.content); break;
                case "use_cases": setTranslatedUseCases(t.content); break;
                case "risks": setTranslatedRisks(t.content); break;
                case "dependencies": setTranslatedDeps(t.content); break;
                case "description": setTranslatedDescription(t.content); break;
              }
            });
            setIsOverviewTranslated(true);
          }
        }).catch(() => {});
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, [open, project.id, aiResult]);

  const loadTags = async () => {
    try {
      const tags = await tagApi.getByProject(project.id);
      setProjectTags(tags);
      const allTagsData = await tagApi.getAll();
      setAllTags(allTagsData);
    } catch (e) {
      console.error(e);
    }
  };

  const handleAddTag = async (tagId: number) => {
    try {
      await tagApi.addToProject(project.id, tagId);
      await loadTags();
      setShowTagSelector(false);
    } catch (e) {
      console.error(e);
    }
  };

  const handleRemoveTag = async (tagId: number) => {
    try {
      await tagApi.removeFromProject(project.id, tagId);
      await loadTags();
    } catch (e) {
      console.error(e);
    }
  };

  const handleGenerateTags = async () => {
    setIsGeneratingTags(true);
    try {
      const result = await invoke<string>("generate_tags", { id: project.id });
      if (result) {
        await loadTags();
      }
    } catch (e) {
      console.error(e);
    } finally {
      setIsGeneratingTags(false);
    }
  };

  const handleTranslateOverview = async () => {
    const hasExistingTranslation = translatedSummary || translatedUseCases || translatedRisks || translatedDeps || translatedDescription;
    if (hasExistingTranslation && !isTranslating) {
      setIsOverviewTranslated(prev => !prev);
      return;
    }

    const hasAiResult = aiResult && (aiResult.summary || aiResult.useCases || aiResult.risks || aiResult.dependencies);
    const hasDescription = !!project.description;
    if (!hasAiResult && !hasDescription) {
      return;
    }

    setIsTranslating(true);
    setIsOverviewTranslated(true);
    const lang = "zh-CN";

    const translateAndSave = async (
      field: string,
      text: string | null | undefined,
      setter: (v: string | null) => void,
      existing: string | null,
    ) => {
      if (!text || text.trim() === "") return;
      if (existing && existing.trim()) return;
      setTranslatingFields(prev => new Set(prev).add(field));
      try {
        const result = await invoke<string>("translate_with_llm", { text, targetLang: lang });
        if (result && result.trim()) {
          setter(result);
          await invoke("save_translation_cmd", { projectId: project.id, fieldName: field, language: lang, content: result }).catch(() => {});
        }
      } catch (e) {
        // 翻译失败，保留原文
      } finally {
        setTranslatingFields(prev => {
          const next = new Set(prev);
          next.delete(field);
          return next;
        });
      }
    };

    const tasks: Promise<void>[] = [];
    if (aiResult?.summary) tasks.push(translateAndSave("summary", aiResult.summary, setTranslatedSummary, translatedSummary));
    if (aiResult?.useCases) tasks.push(translateAndSave("use_cases", aiResult.useCases, setTranslatedUseCases, translatedUseCases));
    if (aiResult?.risks) tasks.push(translateAndSave("risks", aiResult.risks, setTranslatedRisks, translatedRisks));
    if (aiResult?.dependencies) tasks.push(translateAndSave("dependencies", aiResult.dependencies, setTranslatedDeps, translatedDeps));
    if (project.description) tasks.push(translateAndSave("description", project.description, setTranslatedDescription, translatedDescription));

    await Promise.all(tasks);
    setIsTranslating(false);
  };

  const handleTranslateReadme = async () => {
    if (readmeTranslation) {
      setReadmeLang(readmeLang === "original" ? "translated" : "original");
      return;
    }
    if (!project.readme_content) return;
    setIsTranslatingReadme(true);
    try {
      const settings = await invoke<{ defaultLanguage: string }>("get_settings");
      const lang = settings?.defaultLanguage || "zh-CN";
      const translated = await invoke<string>("translate", { text: project.readme_content, targetLang: lang });
      setReadmeTranslation(translated);
      setReadmeLang("translated");
      await saveReadmeTranslation(project.id, translated);
    } catch (e) {
      console.error("README翻译失败:", e);
    } finally {
      setIsTranslatingReadme(false);
    }
  };

const handleAnalyze = useCallback(async () => {
    setAnalyzing(true);
    setTranslatedSummary(null);
    setTranslatedUseCases(null);
    setTranslatedRisks(null);
    setTranslatedDeps(null);
    setTranslatedDescription(null);
    setIsOverviewTranslated(false);
    try {
      await clearTranslations(project.id);
      console.log("[analyze] Starting analyze_project for id:", project.id);
      const startTime = Date.now();
      const result = await invoke<AiResult>("analyze_project", { id: project.id });
      const elapsed = Date.now() - startTime;
      console.log("[analyze] Got result after", elapsed, "ms:", result);
      if (result.error) {
        console.error("[analyze] Error:", result.error);
        showToast(`${t("detail.reanalyzeFailed")}: ${result.error}`, "error");
      } else {
        setAiResult(result);
      }
    } catch (e) {
      console.error("[analyze] Exception:", e);
      showToast(`${t("detail.reanalyzeFailed")}: ${String(e)}`, "error");
    } finally {
      setAnalyzing(false);
      console.log("[analyze] Done, analyzing=false");
    }
  }, [project.id]);

  const languagesList = parseLanguages(project.languages);

  const handleStatusChange = async (newStatus: string) => {
    try {
      await invoke("update_project_status", { id: project.id, lifecycleStatus: newStatus });
      setProject({ ...project, lifecycle_status: newStatus as any });
      onUpdate();
    } catch (e) {
      console.error(e);
    }
  };

  const handleOpenInEditor = async () => {
    if (!project.local_path) return;
    try {
      await invoke("open_in_editor", { path: project.local_path });
    } catch (e) {
      console.error(e);
      showToast(`Failed to open editor: ${e}`, "error");
    }
  };

  const handleAIClassify = async () => {
    setIsClassifying(true);
    try {
      const result = await invoke<{ category_id: number; category_path: string; confidence: number }>("ai_classify_project", { projectId: project.id });
      setProject({ ...project, category_id: result.category_id });
      if (onUpdate) onUpdate();
      showToast(t("detail.aiClassifyDone", { category: result.category_path, confidence: result.confidence }), "success");
    } catch (e) {
      const error = String(e);
      if (error.includes("NO_CATEGORIES")) {
        showToast(t("detail.noCategories"), "error");
      } else if (error.includes("PROJECT_INFO_INSUFFICIENT")) {
        showToast(t("detail.insufficientInfo"), "error");
      } else {
        showToast(`${t("detail.classifyFailed")}: ${e}`, "error");
      }
    } finally {
      setIsClassifying(false);
    }
  };

  const getCategoryPath = (catId: number | null): string => {
    if (!catId) return t("detail.uncategorized");
    const parts: string[] = [];
    let current = categories.find((c) => c.id === catId);
    const visited = new Set<number>();
    while (current && !visited.has(current.id)) {
      visited.add(current.id);
      parts.unshift(current.name);
      current = current.parent_id ? categories.find((c) => c.id === current!.parent_id) : undefined;
    }
    return parts.join(" / ") || t("detail.uncategorized");
  };

  const categoryPath = getCategoryPath(project.category_id);

  const handleDelete = async () => {
    try {
      await deleteProject(project.id, true);
      setShowDeleteModal(false);
      if (onDeleted) {
        onDeleted();
      } else {
        onUpdate();
        onClose();
      }
    } catch (e) {
      showToast(`${t("detail.deleteFailed")}: ${String(e)}`, "error");
    }
  };

  const handleStartEdit = () => {
    setEditName(project.name);
    setEditCategoryId(project.category_id ?? null);
    setEditing(true);
  };

  const handleSave = async () => {
    const trimmedName = editName.trim();
    if (!trimmedName) {
      showToast(t("detail.nameRequired"), "error");
      return;
      return;
    }
    setSaving(true);
    try {
      await invoke("update_project", {
        input: {
          id: project.id,
          name: trimmedName,
          category_id: editCategoryId,
        },
      });
      setEditing(false);
      onUpdate();
      const updated = await invoke<Project>("get_project", { id: project.id });
      setProject(updated);
      setEditName("");
      setEditCategoryId(null);
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setSaving(false);
    }
  };

  const handleCancelEdit = () => {
    setEditing(false);
    setEditName("");
    setEditCategoryId(null);
  };

  const getCategoryDepth = (cat: Category): number => {
    let depth = 0;
    let current: Category | undefined = cat;
    const visited = new Set<number>();
    while (current?.parent_id && !visited.has(current.id)) {
      visited.add(current.id);
      current = categories.find((c) => c.id === current!.parent_id);
      if (!current) break;
      depth++;
    }
    return depth;
  };

  const buildCategoryOptions = (cats: Category[]): { id: number; label: string }[] => {
    const result: { id: number; label: string }[] = [];
    cats.forEach((cat) => {
      const depth = getCategoryDepth(cat);
      const indent = "　".repeat(depth);
      const prefix = depth > 0 ? "├ " : "";
      result.push({ id: cat.id, label: `${indent}${prefix}${cat.name}` });
    });
    const buildPath = (catId: number): string => {
      const parts: string[] = [];
      let current = categories.find((c) => c.id === catId);
      const visited = new Set<number>();
      while (current && !visited.has(current.id)) {
        visited.add(current.id);
        parts.unshift(current.name);
        current = current.parent_id ? categories.find((c) => c.id === current!.parent_id) : undefined;
      }
      return parts.join(" / ") || "";
    };
    result.sort((a, b) => buildPath(a.id).localeCompare(buildPath(b.id), "zh-CN"));
    return result;
  };

  const categoryOptions = buildCategoryOptions(categories.filter((c) => c.name !== t("detail.uncategorized") || c.parent_id !== null));

  if (!open) return null;

  const tabs = [
    { id: "overview" as const, label: t("detail.overview") },
    { id: "ai" as const, label: t("detail.ai") },
    { id: "runbook" as const, label: t("detail.runbook") },
    { id: "readme" as const, label: t("detail.readme") },
    { id: "clone" as const, label: t("detail.clone"), icon: "code-branch" },
    { id: "releases" as const, label: t("detail.releases"), icon: "tag" },
    { id: "notes" as const, label: t("detail.notes") },
    { id: "user" as const, label: t("detail.user"), icon: "lock" },
  ];

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-7xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <header className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <button
                onClick={onClose}
                className="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200"
              >
                <i className="fa-solid fa-arrow-left mr-2"></i>{t("detail.back")}
              </button>
              <div className="w-12 h-12 bg-orange-100 dark:bg-orange-900 rounded-xl flex items-center justify-center text-orange-600 dark:text-orange-400">
                <i className="fa-solid fa-code text-2xl"></i>
              </div>
              <div className="flex-1 min-w-0">
                {editing ? (
                  <input
                    type="text"
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    className="text-xl font-bold border dark:bg-gray-700 dark:border-gray-600 dark:text-gray-200 rounded-lg px-2 py-1 w-full max-w-md"
                    placeholder={t("detail.projectName")}
                    autoFocus
                  />
                ) : (
                  <h1 className="text-xl font-bold dark:text-gray-100">{project.name}</h1>
                )}
                <p className="text-sm text-gray-500 dark:text-gray-400">{project.url}</p>
              </div>
            </div>
            <div className="flex gap-2">
              {(project as any).is_downloaded && (project as any).local_path ? (
                <div className="flex items-center gap-2">
                  {localPathExists === false && (
                    <span className="px-2 py-1 bg-amber-100 dark:bg-amber-900 text-amber-700 dark:text-amber-300 text-xs rounded flex items-center gap-1" title={t("detail.localFileMissingDesc")}>
                      <i className="fa-solid fa-triangle-exclamation"></i>{t("detail.localFileMissing")}
                    </span>
                  )}
                  <button
                    onClick={handleOpenInEditor}
                    disabled={localPathExists === false}
                    className="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    <i className="fa-brands fa-vscode mr-1"></i>{t("detail.openVSCode")}
                  </button>
                </div>
              ) : null}
              {editing ? (
                <>
                  <button
                    onClick={handleSave}
                    disabled={saving}
                    className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
                  >
                    <i className={`fa-solid ${saving ? "fa-spinner fa-spin" : "fa-check"} mr-1`}></i>
                    {saving ? t("detail.saving") : t("detail.save")}
                  </button>
                  <button
                    onClick={handleCancelEdit}
                    disabled={saving}
                    className="px-4 py-2 border dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50"
                  >
                    {t("detail.cancel")}
                  </button>
                </>
              ) : (
                <button
                  onClick={handleStartEdit}
                  className="px-4 py-2 border dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700"
                >
                  <i className="fa-solid fa-pen mr-1"></i>{t("detail.editProject")}
                </button>
              )}
              <button
                onClick={() => setShowDeleteModal(true)}
                className="px-4 py-2 border border-red-300 dark:border-red-800 text-red-600 dark:text-red-400 rounded-lg hover:bg-red-50 dark:hover:bg-red-950"
              >
                <i className="fa-solid fa-trash mr-1"></i>{t("detail.delete")}
              </button>
            </div>
          </div>
        </header>

        {/* Category Bar */}
        <div className="px-6 py-2 bg-gray-50 dark:bg-gray-900 border-b border-gray-200 dark:border-gray-700 flex items-center gap-2 text-sm">
          <span className="text-gray-500 dark:text-gray-400">{t("detail.category")}：</span>
          {editing ? (
            <select
              value={editCategoryId ?? ""}
              onChange={(e) => setEditCategoryId(e.target.value ? Number(e.target.value) : null)}
              className="px-3 py-1 border dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200 rounded text-xs"
            >
              <option value="">{t("detail.uncategorized")}</option>
              {categoryOptions.map((opt) => (
                <option key={opt.id} value={opt.id}>{opt.label}</option>
              ))}
            </select>
          ) : (
            <span className="px-3 py-0.5 bg-indigo-100 text-indigo-700 rounded text-xs font-medium">
              <i className="fa-solid fa-folder mr-1"></i>{categoryPath}
            </span>
          )}
          <button
            onClick={handleAIClassify}
            disabled={isClassifying}
            className="px-3 py-1 text-xs bg-purple-100 text-purple-700 rounded hover:bg-purple-200 flex items-center gap-1 disabled:opacity-75"
          >
            <i className={`fa-solid fa-wand-magic-sparkles ${isClassifying ? "animate-spin" : ""}`}></i>
            {t("detail.aiClassify")}
          </button>
        </div>

        {/* Status Bar */}
        <div className="px-6 py-3 bg-gray-50 dark:bg-gray-900 border-b border-gray-200 dark:border-gray-700 flex items-center gap-4">
          <div className="flex items-center gap-2 text-sm">
            <span className="text-gray-500 dark:text-gray-400">{t("list.status")}：</span>
            {Object.entries(STATUS_CONFIG).map(([key, config]) => (
              <button
                key={key}
                onClick={() => handleStatusChange(key)}
                className={`px-3 py-1 text-xs rounded ${
                  project.lifecycle_status === key
                    ? "bg-indigo-600 text-white"
                    : "bg-white text-gray-700 border border-gray-300 hover:bg-gray-50"
                }`}
              >
                {config.emoji} {t(`status.${key}`)}
              </button>
            ))}
          </div>
        </div>

        {/* Tabs */}
        <div className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6">
          <nav className="flex gap-1">
            {tabs.map((t) => (
              <button
                key={t.id}
                onClick={() => setTab(t.id)}
                className={`px-4 py-3 border-b-2 text-sm ${
                  tab === t.id
                    ? "border-blue-600 text-blue-600 font-medium"
                    : "border-transparent text-gray-500 hover:text-blue-600"
                }`}
              >
                {t.icon === "lock" && <i className="fa-solid fa-lock mr-1"></i>}
                {t.label}
              </button>
            ))}
          </nav>
        </div>

{/* Content */}
        <div className="flex-1 overflow-auto p-6">
          {tab === "overview" && (
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
              <div className="lg:col-span-2 space-y-6">
              {/* AI 加载状态 */}
              {analyzing && (
                <div className="bg-gradient-to-r from-blue-50 to-indigo-50 rounded-xl border border-blue-200 p-6">
                  <div className="flex items-center justify-center py-8">
                    <div className="text-center">
                      <div className="inline-block animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600 mb-3"></div>
                      <p className="text-blue-700 dark:text-blue-300 font-medium">{t("detail.analyzing")}</p>
                      <p className="text-blue-600 dark:text-blue-400 text-sm mt-1">{t("detail.generatingReport")}</p>
                    </div>
                  </div>
                </div>
              )}

              {!analyzing && (
                <div className="space-y-6">
              {/* AI 一句话总结 */}
              <div className="bg-gradient-to-r from-blue-50 to-indigo-50 rounded-xl border border-blue-200 p-6">
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-2">
                    <i className="fa-solid fa-wand-magic-sparkles text-blue-600 dark:text-blue-400"></i>
                    <h2 className="text-lg font-bold text-blue-900 dark:text-blue-200">{t("detail.aiOneLineSummary")}</h2>
                  </div>
                  {translatingFields.has("summary") ? (
                    <span className="text-xs text-blue-600 dark:text-blue-400 bg-blue-100 dark:bg-blue-900 px-2 py-0.5 rounded">
                      <i className="fa-solid fa-spinner fa-spin mr-1"></i>{t("detail.translating")}
                    </span>
                  ) : translatedSummary ? (
                    <span className="text-xs text-blue-600 dark:text-blue-400 bg-blue-100 dark:bg-blue-900 px-2 py-0.5 rounded">{t("detail.translated")}</span>
                  ) : null}
                </div>
                <p className="text-blue-800 dark:text-blue-300 leading-relaxed">
                  {translatingFields.has("summary") ? (
                    <span className="text-gray-400 dark:text-gray-500"><i className="fa-solid fa-spinner fa-spin mr-2"></i>{t("detail.translating")}</span>
                  ) : (
                    (isOverviewTranslated && translatedSummary) || aiResult?.summary || project.description || t("detail.noSummary")
                  )}
                </p>
              </div>

              {/* 项目简介 */}
              <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
                <h2 className="text-lg font-bold dark:text-gray-100 mb-3">{t("detail.projectBrief")}</h2>
                {translatingFields.has("description") ? (
                  <p className="text-gray-400 dark:text-gray-500 text-sm">
                    <i className="fa-solid fa-spinner fa-spin mr-2"></i>{t("detail.translating")}
                  </p>
                ) : (
                  <p className="text-gray-600 dark:text-gray-400 leading-relaxed text-sm">
                    {(isOverviewTranslated && translatedDescription) || project.description || t("detail.noDescription")}
                  </p>
                )}
              </div>

              {/* 适用场景 */}
              {(aiResult?.useCases || (isOverviewTranslated && translatedUseCases)) && (
                <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
                  <h2 className="text-lg font-bold dark:text-gray-100 mb-3">
                    <i className="fa-solid fa-lightbulb text-yellow-500 mr-2"></i>{t("detail.useCases")}
                  </h2>
                  {translatingFields.has("use_cases") ? (
                    <p className="text-gray-400 dark:text-gray-500 text-sm">
                      <i className="fa-solid fa-spinner fa-spin mr-2"></i>{t("detail.translating")}
                    </p>
                  ) : (
                    <p className="text-gray-600 dark:text-gray-400 text-sm whitespace-pre-wrap">
                      {(isOverviewTranslated && translatedUseCases) || aiResult?.useCases}
                    </p>
                  )}
                </div>
              )}

              {/* AI 评估摘要 */}
              {aiResult && aiResult.health_score !== null && (
                <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
                  <h2 className="text-lg font-bold dark:text-gray-100 mb-3">{t("detail.aiEvaluationSummary")}</h2>
                  <div className="space-y-3">
                    <div className="flex items-start gap-3">
                      <div className="w-24 text-sm text-gray-500 dark:text-gray-400">{t("detail.health")}</div>
                      <div className="flex-1">
                        <div className="flex items-center gap-2">
                          <div className="flex-1 bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                            <div
                              className={`h-2 rounded-full ${
                                aiResult.health_score >= 80 ? "bg-green-500" :
                                aiResult.health_score >= 60 ? "bg-blue-500" :
                                aiResult.health_score >= 40 ? "bg-yellow-500" : "bg-red-500"
                              }`}
                              style={{ width: `${aiResult.health_score}%` }}
                            ></div>
                          </div>
                          <span className="text-sm font-medium dark:text-gray-100">{Math.round(aiResult.health_score)}</span>
                        </div>
                      </div>
                    </div>
                    {(aiResult.risks || (isOverviewTranslated && translatedRisks)) && (
                      <div className="flex items-start gap-3">
                        <div className="w-24 text-sm text-gray-500 dark:text-gray-400">{t("detail.risks")}</div>
                        <div className="flex-1 text-sm text-orange-600 dark:text-orange-400">
                          {translatingFields.has("risks") ? (
                            <span><i className="fa-solid fa-spinner fa-spin mr-1"></i>{t("detail.translating")}</span>
                          ) : (
                            (isOverviewTranslated && translatedRisks) || aiResult.risks
                          )}
                        </div>
                      </div>
                    )}
                    {(aiResult.dependencies || (isOverviewTranslated && translatedDeps)) && (
                      <div className="flex items-start gap-3">
                        <div className="w-24 text-sm text-gray-500 dark:text-gray-400">{t("detail.dependencies")}</div>
                        <div className="flex-1 text-sm text-gray-600 dark:text-gray-400">
                          {translatingFields.has("dependencies") ? (
                            <span><i className="fa-solid fa-spinner fa-spin mr-1"></i>{t("detail.translating")}</span>
                          ) : (
                            (isOverviewTranslated && translatedDeps) || aiResult.dependencies
                          )}
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              )}
              </div>
              )}
              </div>

              {/* 右侧边栏 */}
              <div className="space-y-6">
              {/* 元数据 */}
              <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
                <h2 className="text-lg font-bold dark:text-gray-100 mb-3">{t("detail.metadata")}</h2>
                <ul className="space-y-3 text-sm">
                  <li className="flex justify-between"><span className="text-gray-500 dark:text-gray-400">{t("detail.stars")}</span><span className="dark:text-gray-100">★ {project.stars}</span></li>
                  {project.forks > 0 && <li className="flex justify-between"><span className="text-gray-500 dark:text-gray-400">{t("detail.forks")}</span><span className="dark:text-gray-100">{project.forks}</span></li>}
                  {project.license && <li className="flex justify-between"><span className="text-gray-500 dark:text-gray-400">{t("detail.license")}</span><span className="text-green-600 dark:text-green-400">{project.license}</span></li>}
                  {languagesList.length > 0 && (
                    <li className="flex justify-between items-start"><span className="text-gray-500 dark:text-gray-400">{t("detail.language")}</span>
                      <div className="flex flex-wrap gap-1 justify-end">
                        {languagesList.slice(0, 3).map((lang, i) => (
                          <span key={i} className="px-1.5 py-0.5 bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 text-xs rounded">{lang}</span>
                        ))}
                      </div>
                    </li>
                  )}
                  <li className="flex justify-between"><span className="text-gray-500 dark:text-gray-400">{t("detail.source")}</span><span className="dark:text-gray-100">{project.source}</span></li>
                </ul>
              </div>

              {/* AI 操作 */}
              <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
                <div className="flex items-center justify-between mb-3">
                  <h2 className="text-lg font-bold dark:text-gray-100">{t("detail.ai")}</h2>
                  {backgroundTasks.length > 0 && (
                    <span className="text-xs text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-950 px-2 py-1 rounded-full flex items-center gap-1">
                      <i className="fa-solid fa-spinner fa-spin"></i>
                      {t("detail.backgroundTasks", { count: backgroundTasks.length })}
                    </span>
                  )}
                </div>
                {backgroundTasks.length > 0 && (
                  <div className="mb-3 p-3 bg-blue-50 dark:bg-blue-950 rounded-lg space-y-1.5">
                    {backgroundTasks.map((task, i) => (
                      <div key={i} className="flex items-center gap-2 text-xs text-blue-700 dark:text-blue-300">
                        <i className="fa-solid fa-spinner fa-spin"></i>
                        <span>
                          {task.taskType === "ai_analysis" && t("detail.taskAiAnalysis")}
                          {task.taskType === "generate_tags" && t("detail.taskGenerateTags")}
                          {task.taskType === "description" && t("detail.taskDescription")}
                          {task.taskType === "readme_translation" && t("detail.taskReadmeTranslation")}
                          {!["ai_analysis", "generate_tags", "description", "readme_translation"].includes(task.taskType) && t("detail.taskGeneric", { type: task.taskType })}
                        </span>
                      </div>
                    ))}
                    <p className="text-xs text-blue-500 dark:text-blue-400 mt-1">{t("detail.backgroundTaskNote")}</p>
                  </div>
                )}
                <div className="space-y-2">
                  <button onClick={handleAnalyze} disabled={analyzing} className="w-full px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50">
                    <i className={`fa-solid ${analyzing ? "fa-spinner fa-spin" : "fa-wand-magic-sparkles"} mr-1`}></i>
                    {analyzing ? "分析中..." : "重新分析"}
                  </button>
                  <button onClick={handleTranslateOverview} disabled={isTranslating || (!aiResult && !project.description && !translatedDescription)} className="w-full px-4 py-2 border dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50">
                    <i className={`fa-solid ${isOverviewTranslated ? "fa-rotate-left" : "fa-language"} mr-1`}></i>
                    {isOverviewTranslated ? "显示原文" : "翻译概览"}
                  </button>
                  <button onClick={handleGenerateTags} disabled={isGeneratingTags} className="w-full px-4 py-2 border dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50">
                    <i className={`fa-solid ${isGeneratingTags ? "fa-spinner fa-spin" : "fa-tags"} mr-1`}></i>
                    {isGeneratingTags ? "打标签中..." : "AI打标签"}
                  </button>
                </div>
              </div>

              {/* 标签 */}
              <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
                <div className="flex items-center justify-between mb-3">
                  <h2 className="text-lg font-bold dark:text-gray-100">{t("detail.tags")}</h2>
                  <button onClick={() => setShowTagSelector(!showTagSelector)} className="px-3 py-1 text-sm text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-950 rounded-lg">
                    <i className="fa-solid fa-plus mr-1"></i>{t("detail.addTag")}
                  </button>
                </div>
                {projectTags.length > 0 ? (
                  <div className="flex flex-wrap gap-2">
                    {projectTags.map((tag) => (
                      <span key={tag.id} className="px-2 py-1 bg-indigo-100 text-indigo-700 text-sm rounded-full flex items-center gap-1">
                        {tag.name}
                        <button onClick={() => handleRemoveTag(tag.id)} className="hover:text-red-600">
                          <i className="fa-solid fa-times text-xs"></i>
                        </button>
                      </span>
                    ))}
                  </div>
                ) : (
                  <p className="text-xs text-gray-500 dark:text-gray-400">暂无标签，点击"添加"或"AI打标签"</p>
                )}
                {showTagSelector && (
                  <div className="mt-3 pt-3 border-t dark:border-gray-700">
                    <div className="flex flex-wrap gap-2 max-h-32 overflow-y-auto">
                      {allTags.filter(t => !projectTags.find(pt => pt.id === t.id)).map((tag) => (
                        <button key={tag.id} onClick={() => handleAddTag(tag.id)} className="px-2 py-1 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 text-gray-700 dark:text-gray-300 text-sm rounded">
                          <i className="fa-solid fa-plus mr-1"></i>{tag.name}
                        </button>
                      ))}
                      {allTags.filter(t => !projectTags.find(pt => pt.id === t.id)).length === 0 && (
                        <p className="text-xs text-gray-400 dark:text-gray-500">{t("detail.noTags")}</p>
                      )}
                    </div>
                  </div>
                )}
              </div>

              {/* 项目操作 */}
              <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
                <h2 className="text-lg font-bold dark:text-gray-100 mb-3">{t("detail.operations")}</h2>
                <div className="space-y-2">
                  {(project as any).local_path && (
                    <>
                      {localPathExists === false && (
                        <div className="px-3 py-2 bg-amber-50 dark:bg-amber-950 border border-amber-200 dark:border-amber-800 rounded-lg text-xs text-amber-700 dark:text-amber-300 flex items-start gap-2">
                          <i className="fa-solid fa-triangle-exclamation mt-0.5"></i>
                          <div>
                            <p className="font-medium">{t("detail.localFileMissing")}</p>
                            <p className="text-amber-600 dark:text-amber-400 mt-0.5 break-all">{(project as any).local_path}</p>
                          </div>
                        </div>
                      )}
                      <button
                        onClick={handleOpenInEditor}
                        disabled={localPathExists === false}
                        className="w-full px-4 py-2 border dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-1"
                      >
                        <i className="fa-solid fa-code"></i>{t("detail.openInEditor")}
                        {localPathExists === true && <i className="fa-solid fa-circle-check text-green-500 text-xs ml-1"></i>}
                      </button>
                    </>
                  )}
                  <button className="w-full px-4 py-2 border dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 text-red-600 dark:text-red-400">
                    <i className="fa-solid fa-trash mr-1"></i>{t("detail.deleteProject")}
                  </button>
                </div>
              </div>
              </div>
            </div>
          )}

          {tab === "ai" && (
            <div className="space-y-6">
              {analyzing && (
                <div className="text-center py-12">
                  <div className="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
                  <p className="mt-4 text-gray-500 dark:text-gray-400">{t("detail.analyzing")}</p>
                </div>
              )}

              {!analyzing && !aiResult && (
                <div className="text-center py-12">
                  <button onClick={handleAnalyze} className="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700">
                    <i className="fa-solid fa-wand-magic-sparkles mr-2"></i>{t("detail.analyzeProject")}
                  </button>
                </div>
              )}

              {aiResult && (
                <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
                  <div className="flex items-center justify-between mb-6">
                    <h2 className="text-lg font-bold dark:text-gray-100">AI 深度体检报告</h2>
                    <div className="flex items-center gap-2">
                      {(translatedSummary || translatedUseCases || translatedRisks) && (
                        <span className="text-xs text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-950 px-2 py-0.5 rounded">{t("detail.translated")}</span>
                      )}
                      <button
                        onClick={async () => {
                          try {
                            const result = await testLlmDirect(project.id);
                            console.log("test_llm_direct result:", result);
                            showToast(result.success ? `测试成功 (${result.elapsed_ms}ms)` : `测试失败: ${result.error}`, result.success ? "success" : "error");
                          } catch (e) {
                            console.error("test_llm_direct error:", e);
                            showToast(`测试失败: ${e}`, "error");
                          }
                        }}
                        className="px-2 py-1 text-xs bg-gray-500 text-white rounded hover:bg-gray-600"
                      >
                        <i className="fa-solid fa-bug mr-1"></i>调试
                      </button>
                      <button
                        onClick={handleAnalyze}
                        className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                      >
                        <i className="fa-solid fa-wand-magic-sparkles mr-1"></i>重新分析
                      </button>
                    </div>
                  </div>
                  <div className="space-y-6">
                    {/* 健康度分析 */}
                    <div>
                      <h3 className="font-medium text-green-600 dark:text-green-400 mb-2">✓ 健康度分析</h3>
                      <ul className="list-disc list-inside text-sm text-gray-600 dark:text-gray-400 space-y-1 pl-4">
                        <li>综合评分：{aiResult.health_score}/100（{aiResult.health_rating}）</li>
                        <li>{translatedSummary || aiResult.summary || "暂无"}</li>
                      </ul>
                    </div>
                    {/* License 合规 */}
                    {project.license && (
                      <div>
                        <h3 className="font-medium text-blue-600 dark:text-blue-400 mb-2">🔒 License 合规</h3>
                        <p className="text-sm text-gray-600 dark:text-gray-400">{project.license} 许可证，商业友好。</p>
                      </div>
                    )}
                    {/* 适用场景 */}
                    {(aiResult.useCases || translatedUseCases) && (
                      <div>
                        <h3 className="font-medium text-purple-600 mb-2">💬 {t("detail.useCases")}</h3>
                        <p className="text-sm text-gray-600 dark:text-gray-400 whitespace-pre-wrap">{translatedUseCases || aiResult.useCases}</p>
                      </div>
                    )}
                    {/* 风险提示 */}
                    {(aiResult.risks || translatedRisks) && (
                      <div>
                        <h3 className="font-medium text-yellow-600 mb-2">⚠️ 安全提示</h3>
                        <p className="text-sm text-gray-600 dark:text-gray-400 whitespace-pre-wrap">{translatedRisks || aiResult.risks}</p>
                      </div>
                    )}
                    {/* 依赖信息 */}
                    {(aiResult.dependencies || translatedDeps) && (
                      <div>
                        <h3 className="font-medium text-gray-600 dark:text-gray-400 mb-2">📦 依赖信息</h3>
                        <p className="text-sm text-gray-600 dark:text-gray-400 whitespace-pre-wrap">{translatedDeps || aiResult.dependencies}</p>
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          )}

          {tab === "runbook" && (
            <div className="space-y-4">
              {analyzing ? (
                <div className="text-center py-12">
                  <div className="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
                  <p className="mt-4 text-gray-500 dark:text-gray-400">AI 正在生成 Runbook...</p>
                </div>
              ) : backgroundTasks.some((t) => t.taskType === "runbook") ? (
                <div className="text-center py-12">
                  <div className="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
                  <p className="mt-4 text-gray-500 dark:text-gray-400">后台正在生成 Runbook...</p>
                  <p className="text-sm text-gray-400 dark:text-gray-500 mt-1">生成完成后自动显示</p>
                </div>
              ) : (
                <>
                  <div className="flex items-center justify-between">
                    <h2 className="text-lg font-bold dark:text-gray-100">快速上手指南</h2>
                    <div className="flex gap-2">
                      {isEditingRunbook ? (
                        <>
                          <button
                            onClick={async () => {
                              try {
                                await saveRunbook(project.id, editedRunbook);
                                setRunbookContent(editedRunbook);
                                setIsEditingRunbook(false);
                              } catch (e) {
                                console.error(e);
                                showToast("保存失败: " + e, "error");
                              }
                            }}
                            className="px-3 py-1.5 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700"
                          >
                            {t("detail.saveRunbook")}
                          </button>
                          <button
                            onClick={() => {
                              setIsEditingRunbook(false);
                              setEditedRunbook(runbookContent || "");
                            }}
                            className="px-3 py-1.5 text-sm border dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700"
                          >
                            {t("common.cancel")}
                          </button>
                        </>
                      ) : (
                        <>
                          {runbookContent && (
                            <button
                              onClick={() => {
                                setEditedRunbook(runbookContent || "");
                                setIsEditingRunbook(true);
                              }}
                              className="px-3 py-1.5 text-sm border dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700"
                            >
                              <i className="fa-solid fa-pen mr-1"></i>
                              {t("detail.editRunbook")}
                            </button>
                          )}
                          <button
                            onClick={async () => {
                              setAnalyzing(true);
                              try {
                                const result = await invoke<string>("generate_runbook", { id: project.id });
                                if (result) {
                                  setRunbookContent(result);
                                  try {
                                    await saveRunbook(project.id, result);
                                  } catch (e) {
                                    console.error("Failed to save runbook:", e);
                                  }
                                }
                              } catch (e) {
                                console.error(e);
                              } finally {
                                setAnalyzing(false);
                              }
                            }}
                            className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                          >
                            <i className="fa-solid fa-wand-magic-sparkles mr-1"></i>
                            {runbookContent ? "重新生成" : "AI 生成"}
                          </button>
                        </>
                      )}
                    </div>
                  </div>
                  {runbookContent ? (
                    isEditingRunbook ? (
                      <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4">
                        <textarea
                          value={editedRunbook}
                          onChange={(e) => setEditedRunbook(e.target.value)}
                          className="w-full h-96 p-3 border dark:bg-gray-700 dark:border-gray-600 dark:text-gray-200 rounded-lg font-mono text-sm"
                          placeholder="输入 Markdown 格式的笔记..."
                        />
                      </div>
                    ) : (
                      <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
                        <MarkdownRenderer content={runbookContent} className="prose dark:prose-invert prose-sm max-w-none readme-content text-sm" baseUrl={project.url || undefined} />
                      </div>
                    )
                  ) : (
                    <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
                      <div className="space-y-4 text-sm dark:text-gray-300">
                        <div>
                          <h4 className="font-medium mb-2 dark:text-gray-100">1. 克隆项目</h4>
                          <code className="block bg-gray-100 dark:bg-gray-700 p-3 rounded font-mono">
                            git clone {project.url || "https://github.com/user/repo"}
                          </code>
                        </div>
                        <div>
                          <h4 className="font-medium mb-2 dark:text-gray-100">2. 安装依赖</h4>
                          <code className="block bg-gray-100 dark:bg-gray-700 p-3 rounded font-mono">
                            npm install
                          </code>
                        </div>
                        <div>
                          <h4 className="font-medium mb-2 dark:text-gray-100">3. 运行项目</h4>
                          <code className="block bg-gray-100 dark:bg-gray-700 p-3 rounded font-mono">
                            npm run dev
                          </code>
                        </div>
                      </div>
                    </div>
                  )}
                </>
              )}
            </div>
          )}

          {tab === "readme" && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <h2 className="text-lg font-bold dark:text-gray-100">README</h2>
                  <select
                    className="px-3 py-1 border dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200 rounded-lg text-sm"
                    value={selectedVariant || "original"}
                    onChange={(e) => setSelectedVariant(e.target.value === "original" ? null : e.target.value)}
                  >
                    <option value="original">默认 README</option>
                    {readmeVariants.map((v) => (
                      <option key={v.id} value={v.file_name}>
                        {v.file_name} ({v.language})
                      </option>
                    ))}
                  </select>
                </div>
                  <button onClick={async () => {
                    setRefreshing(true);
                    try {
                      const result = await refreshProjectReadme(project.id);
                      if (result.readme_content) {
                        setProject((p: any) => ({ ...p, readme_content: result.readme_content }));
                      }
                      await invoke<any[]>("get_readme_variants_cmd", { projectId: project.id })
                        .then(setReadmeVariants)
                        .catch(() => {});
                      showToast("README 刷新成功", "success");
                    } catch (e) {
                      showToast(`刷新失败: ${e}`, "error");
                    } finally {
                      setRefreshing(false);
                    }
                  }} className="px-3 py-1 text-sm border dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700" disabled={refreshing}>
                    <i className={`fa-solid fa-rotate ${refreshing ? "fa-spin" : ""} mr-1`}></i>{refreshing ? "刷新中..." : "刷新"}
                  </button>
              </div>

              {/* 翻译按钮 - 始终显示 */}
              <div className="flex items-center gap-2 bg-gray-100 dark:bg-gray-700 p-1 rounded-lg w-fit">
                <button
                  onClick={() => setReadmeLang("original")}
                  className={`px-4 py-2 text-sm rounded-md transition-colors ${
                    readmeLang === "original" ? "bg-white dark:bg-gray-800 shadow-sm text-blue-600 dark:text-blue-400 font-medium" : "text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-gray-100"
                  }`}
                >
                  {t("detail.original")}
                </button>
                <button
                  onClick={async () => {
                    if (!readmeTranslation) {
                      await handleTranslateReadme();
                    } else {
                      setReadmeLang("translated");
                    }
                  }}
                  disabled={isTranslatingReadme || !project.readme_content}
                  className={`px-4 py-2 text-sm rounded-md transition-colors disabled:opacity-50 ${
                    readmeLang === "translated" ? "bg-white dark:bg-gray-800 shadow-sm text-blue-600 dark:text-blue-400 font-medium" : "text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-gray-100"
                  }`}
                >
                  {isTranslatingReadme ? (
                    <><i className="fa-solid fa-spinner fa-spin mr-1"></i>{t("detail.translating")}</>
                  ) : t("detail.translation")}
                </button>
              </div>

              {/* 显示选中的 README 版本 */}
              {(() => {
                const selectedReadme = selectedVariant
                  ? readmeVariants.find(v => v.file_name === selectedVariant)
                  : null;
                const defaultContent = selectedReadme?.content || project.readme_content;
                const content = (readmeLang === "translated" && readmeTranslation) ? readmeTranslation : defaultContent;

                if (backgroundTasks.some((t) => t.taskType === "readme_translation") && !content) {
                  return (
                    <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6 text-center">
                      <div className="inline-block animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600 mb-3"></div>
                      <p className="text-gray-500 dark:text-gray-400">后台正在翻译 README...</p>
                      <p className="text-sm text-gray-400 dark:text-gray-500 mt-1">翻译完成后自动显示</p>
                    </div>
                  );
                }

                if (content) {
                  return (
                    <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6 overflow-auto max-h-[60vh]">
                      <MarkdownRenderer content={content} className="text-sm" baseUrl={project.url || undefined} />
                    </div>
                  );
                } else {
                  return (
                    <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6 text-center">
                      <i className="fa-solid fa-file-lines text-4xl text-gray-300 dark:text-gray-600 mb-4"></i>
                      <p className="text-gray-500 dark:text-gray-400">暂无 README 内容</p>
                    </div>
                  );
                }
              })()}
            </div>
          )}

          {tab === "notes" && (
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-bold dark:text-gray-100">{t("detail.notes")}</h2>
              </div>
              <div className="space-y-4">
                {projectNotes.length > 0 ? (
                  projectNotes.map((note) => (
                    <div key={note.id} className="bg-blue-50 dark:bg-blue-950 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
                      <div className="flex items-center justify-between mb-2">
                        <span className="text-sm font-medium text-blue-800 dark:text-blue-300">{new Date(note.created_at).toLocaleString()}</span>
                        <div className="flex gap-2">
                          <button
                            onClick={async () => {
                              try {
                                await noteApi.delete(note.id);
                                setProjectNotes(projectNotes.filter(n => n.id !== note.id));
                              } catch (e) {
                                console.error(e);
                              }
                            }}
                            className="text-red-500 hover:text-red-700"
                          >
                            <i className="fa-solid fa-trash"></i>
                          </button>
                        </div>
                      </div>
                      <p className="text-sm text-blue-900 dark:text-blue-200 whitespace-pre-wrap">{note.content}</p>
                    </div>
                  ))
                ) : (
                  <p className="text-sm text-gray-500 dark:text-gray-400 text-center py-4">{t("detail.noNotes")}</p>
                )}
              </div>
              <div className="mt-4 pt-4 border-t dark:border-gray-700">
                <textarea
                  value={noteInput}
                  onChange={(e) => setNoteInput(e.target.value)}
                  placeholder="记录你的使用心得、踩坑记录..."
                  className="w-full px-4 py-3 border border-gray-300 dark:bg-gray-800 dark:border-gray-600 dark:text-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                  rows={3}
                />
                <button
                  onClick={async () => {
                    if (!noteInput.trim()) return;
                    try {
                      await noteApi.create({ project_id: project.id, content: noteInput });
                      const notes = await noteApi.getByProject(project.id);
                      setProjectNotes(notes);
                      setNoteInput("");
                    } catch (e) {
                      console.error(e);
                      showToast("保存失败: " + e, "error");
                    }
                  }}
                  disabled={!noteInput.trim()}
                  className="mt-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 text-sm"
                >
                  保存笔记
                </button>
              </div>
            </div>
          )}

          {tab === "clone" && <ClonePanel project={project} />}

          {tab === "releases" && <ReleasesPanel project={project} />}

          {tab === "user" && (
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6 text-center">
              <i className="fa-solid fa-lock text-4xl text-gray-300 dark:text-gray-600 mb-4"></i>
              <h2 className="text-lg font-bold dark:text-gray-100 mb-2">{t("detail.user")}</h2>
              <p className="text-gray-500 dark:text-gray-400">用于存储项目相关的敏感信息（如 Token、密钥等）</p>
              <p className="text-sm text-gray-400 dark:text-gray-500 mt-4">功能开发中...</p>
            </div>
          )}
        </div>
      </div>

      {/* 删除确认弹框 */}
      {showDeleteModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-[60]">
          <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-md p-6 mx-4">
            <div className="flex items-start gap-4 mb-4">
              <div className="w-12 h-12 bg-red-100 dark:bg-red-900 rounded-full flex items-center justify-center text-red-600 dark:text-red-400 flex-shrink-0">
                <i className="fa-solid fa-triangle-exclamation text-xl"></i>
              </div>
              <div>
                <h3 className="text-lg font-bold text-gray-900 dark:text-gray-100 mb-1">确认删除项目？</h3>
                <p className="text-sm text-gray-600 dark:text-gray-300">
                  项目 <span className="font-medium text-gray-900 dark:text-gray-100">"{project.name}"</span> 及其所有数据（AI 分析、翻译、笔记、标签）将被永久删除。
                </p>
                <p className="text-sm text-red-600 dark:text-red-400 mt-2 font-medium">此操作不可恢复！</p>
              </div>
            </div>
            <div className="flex justify-end gap-3 mt-6">
              <button
                onClick={() => setShowDeleteModal(false)}
                className="px-4 py-2 border dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700"
              >
                {t("common.cancel")}
              </button>
              <button
                onClick={handleDelete}
                className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700"
              >
                <i className="fa-solid fa-trash mr-1"></i>{t("common.delete")}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Toast Container */}
      {toasts.length > 0 && (
        <div className="fixed bottom-4 right-4 z-50 space-y-2">
          {toasts.map(t => (
            <div
              key={t.id}
              className={`px-4 py-3 rounded-lg shadow-lg flex items-center gap-2 animate-slide-in ${
                t.type === "success" ? "bg-green-500 text-white" :
                t.type === "error" ? "bg-red-500 text-white" :
                "bg-gray-800 text-white"
              }`}
            >
              <i className={`fa-solid ${
                t.type === "success" ? "fa-check-circle" :
                t.type === "error" ? "fa-circle-exclamation" :
                "fa-info-circle"
              }`}></i>
              {t.message}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
