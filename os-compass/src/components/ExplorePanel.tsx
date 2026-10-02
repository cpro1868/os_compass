import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  askExplore,
  deleteExploreSession,
  getExploreKbStatus,
  getExploreMessages,
  getLatestExploreSession,
  listExploreSessions,
  type ExploreKbStatus,
  type ExploreMessage,
  type ExploreSession,
  type ExploreSource,
} from "../api/explore";
import { noteApi } from "../api/note";
import { useToastStore } from "../stores/toastStore";
import { MarkdownRenderer } from "./MarkdownRenderer";

interface ExplorePanelProps {
  projectId: number;
  projectName: string;
}

interface ChatItem {
  role: "user" | "assistant";
  content: string;
  sources?: ExploreSource[];
  error?: boolean;
}

function parseSources(json: string | null | undefined): ExploreSource[] {
  if (!json) return [];
  try {
    const arr = JSON.parse(json);
    return Array.isArray(arr) ? arr : [];
  } catch {
    return [];
  }
}

function SourceBadges({ sources }: { sources: ExploreSource[] }) {
  const { t } = useTranslation();
  if (!sources.length) return null;
  const readmeCount = sources.filter((s) => s.type === "readme_chunk").length;
  const badges: string[] = [];
  if (readmeCount > 0) badges.push(`${t("detail.exploreSourceReadme")} ×${readmeCount}`);
  if (sources.some((s) => s.type === "ai_report")) badges.push(t("detail.exploreSourceAi"));
  if (sources.some((s) => s.type === "runbook")) badges.push(t("detail.exploreSourceRunbook"));
  if (sources.some((s) => s.type === "notes")) badges.push(t("detail.exploreSourceNotes"));
  return (
    <div className="mt-2 flex flex-wrap items-center gap-1.5">
      <span className="text-xs text-gray-400 dark:text-gray-500">{t("detail.exploreSources")}:</span>
      {badges.map((b) => (
        <span
          key={b}
          className="rounded-full bg-gray-100 dark:bg-gray-700 px-2 py-0.5 text-xs text-gray-600 dark:text-gray-300"
        >
          {b}
        </span>
      ))}
    </div>
  );
}

export function ExplorePanel({ projectId, projectName }: ExplorePanelProps) {
  const { t } = useTranslation();
  const { showToast } = useToastStore();
  const [sessions, setSessions] = useState<ExploreSession[]>([]);
  const [currentSessionId, setCurrentSessionId] = useState<number | undefined>(undefined);
  const [messages, setMessages] = useState<ChatItem[]>([]);
  const [input, setInput] = useState("");
  const [asking, setAsking] = useState(false);
  const [kbStatus, setKbStatus] = useState<ExploreKbStatus | null>(null);
  const [lastQuestion, setLastQuestion] = useState("");
  const bottomRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const welcomeItem: ChatItem = {
    role: "assistant",
    content: kbStatus && !kbStatus.ready && !kbStatus.hasAiReport && kbStatus.notesCount === 0
      ? t("detail.exploreWelcomeEmpty")
      : t("detail.exploreWelcome"),
  };

  const loadSessionMessages = useCallback(async (sessionId: number) => {
    const msgs: ExploreMessage[] = await getExploreMessages(sessionId);
    setMessages(
      msgs.map((m) => ({
        role: m.role,
        content: m.content,
        sources: m.role === "assistant" ? parseSources(m.sources) : undefined,
      }))
    );
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const [latest, list, kb] = await Promise.all([
        getLatestExploreSession(projectId).catch(() => null),
        listExploreSessions(projectId).catch(() => [] as ExploreSession[]),
        getExploreKbStatus(projectId).catch(() => null),
      ]);
      if (cancelled) return;
      setSessions(list);
      setKbStatus(kb);
      if (latest) {
        setCurrentSessionId(latest.id);
        await loadSessionMessages(latest.id);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [projectId, loadSessionMessages]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView?.({ behavior: "smooth" });
  }, [messages, asking]);

  const handleAsk = useCallback(
    async (question?: string) => {
      const q = (question ?? input).trim();
      if (!q || asking) return;
      setLastQuestion(q);
      setInput("");
      if (inputRef.current) inputRef.current.style.height = "auto";
      setAsking(true);
      // 本次请求的局部消息序列，避免依赖 render 间共享 state
      const base: ChatItem[] = [...messages, { role: "user", content: q }];
      setMessages(base);
      try {
        const res = await askExplore(projectId, currentSessionId, q);
        if (currentSessionId === undefined) setCurrentSessionId(res.sessionId);
        setMessages([...base, { role: "assistant", content: res.answer, sources: res.sources }]);
        if (currentSessionId === undefined) {
          const list = await listExploreSessions(projectId).catch(() => null);
          if (list) setSessions(list);
        }
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        setMessages([...base, { role: "assistant", content: msg, error: true }]);
      } finally {
        setAsking(false);
      }
    },
    [asking, input, messages, projectId, currentSessionId]
  );

  const handleSaveNote = useCallback(
    async (idx: number) => {
      const item = messages[idx];
      if (!item || item.role !== "assistant") return;
      // 找该回答对应的用户问题
      let question = "";
      for (let i = idx - 1; i >= 0; i--) {
        if (messages[i].role === "user") {
          question = messages[i].content;
          break;
        }
      }
      const title = question.length > 30 ? question.slice(0, 30) : question;
      try {
        await noteApi.create({
          project_id: projectId,
          content: `\n\n---\n\n## 探索问答：${title}\n\n${item.content}`,
        });
        showToast(t("detail.exploreSaved"), "success");
      } catch {
        showToast(t("detail.exploreSaveFailed"), "error");
      }
    },
    [messages, projectId, showToast, t]
  );

  const handleCopy = useCallback(
    async (content: string) => {
      try {
        await navigator.clipboard.writeText(content);
        showToast(t("detail.exploreCopied"), "success");
      } catch {
        // 剪贴板不可用时静默
      }
    },
    [showToast, t]
  );

  const handleSelectSession = useCallback(
    async (id: number) => {
      if (id === currentSessionId || asking) return;
      setCurrentSessionId(id);
      await loadSessionMessages(id);
    },
    [currentSessionId, asking, loadSessionMessages]
  );

  const handleNewSession = useCallback(() => {
    if (asking) return;
    setCurrentSessionId(undefined);
    setMessages([]);
  }, [asking]);

  const handleDeleteSession = useCallback(
    async (id: number) => {
      if (!window.confirm(t("detail.exploreDeleteConfirm"))) return;
      await deleteExploreSession(id).catch(() => {});
      setSessions((prev) => prev.filter((s) => s.id !== id));
      if (id === currentSessionId) {
        setCurrentSessionId(undefined);
        setMessages([]);
      }
    },
    [currentSessionId, t]
  );

  const autoResize = (el: HTMLTextAreaElement) => {
    el.style.height = "auto";
    el.style.height = Math.min(el.scrollHeight, 120) + "px";
  };

  const showWelcome = messages.length === 0 && !asking;
  const suggestions = [
    t("detail.exploreSuggest1"),
    t("detail.exploreSuggest2"),
    t("detail.exploreSuggest3"),
    t("detail.exploreSuggest4"),
  ];

  return (
    <div className="flex h-full min-h-0 gap-4">
      {/* 会话列表 */}
      <div className="w-52 shrink-0 flex flex-col rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900">
        <div className="flex items-center justify-between px-3 py-2 border-b border-gray-200 dark:border-gray-700">
          <span className="text-sm font-medium text-gray-700 dark:text-gray-200">
            {t("detail.exploreSessions")}
          </span>
          <button
            onClick={handleNewSession}
            title={t("detail.exploreNewSession")}
            className="rounded p-1 text-gray-500 hover:bg-gray-200 dark:hover:bg-gray-700 dark:text-gray-400"
          >
            <i className="fa-solid fa-plus" />
          </button>
        </div>
        <div className="flex-1 overflow-y-auto p-1.5 space-y-1">
          {sessions.map((s) => (
            <div
              key={s.id}
              onClick={() => handleSelectSession(s.id)}
              className={`group flex cursor-pointer items-center justify-between rounded px-2 py-1.5 text-sm ${
                s.id === currentSessionId
                  ? "bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300"
                  : "text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700"
              }`}
            >
              <span className="truncate">{s.title || `#${s.id}`}</span>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleDeleteSession(s.id);
                }}
                className="invisible ml-1 text-gray-400 hover:text-red-500 group-hover:visible"
              >
                <i className="fa-solid fa-xmark" />
              </button>
            </div>
          ))}
        </div>
      </div>

      {/* 对话区 */}
      <div className="flex min-w-0 flex-1 flex-col">
        {/* 状态条 */}
        {kbStatus && (
          <div
            className={`mb-2 flex items-center gap-2 rounded-lg px-3 py-1.5 text-xs ${
              kbStatus.degraded
                ? "bg-amber-50 dark:bg-amber-900/20 text-amber-700 dark:text-amber-400"
                : "bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-400"
            }`}
          >
            <i className={`fa-solid ${kbStatus.degraded ? "fa-triangle-exclamation" : "fa-circle-check"}`} />
            <span>
              {kbStatus.degraded
                ? t("detail.exploreKbDegraded")
                : kbStatus.ready
                  ? t("detail.exploreKbReady")
                  : t("detail.exploreKbEmpty")}
            </span>
            {kbStatus.degraded && (
              <button
                onClick={() => window.dispatchEvent(new CustomEvent("navigate", { detail: { openSettings: true } }))}
                className="ml-auto underline hover:no-underline"
              >
                {t("detail.exploreGoSettings")}
              </button>
            )}
          </div>
        )}

        {/* 消息流 */}
        <div className="flex-1 min-h-0 overflow-y-auto rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4 space-y-4">
          {showWelcome && (
            <div>
              <div className="rounded-lg bg-gray-50 dark:bg-gray-700/50 p-4 text-sm text-gray-700 dark:text-gray-200">
                <MarkdownRenderer content={welcomeItem.content} />
                <div className="mt-1 text-xs text-gray-400 dark:text-gray-500">
                  {t("detail.exploreBasedOn")}
                </div>
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                {suggestions.map((s) => (
                  <button
                    key={s}
                    onClick={() => handleAsk(s)}
                    className="rounded-full border border-blue-200 dark:border-blue-800 px-3 py-1 text-xs text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/30"
                  >
                    {s}
                  </button>
                ))}
              </div>
            </div>
          )}
          {messages.map((m, idx) =>
            m.role === "user" ? (
              <div key={idx} className="flex justify-end">
                <div className="max-w-[80%] rounded-lg bg-blue-600 px-3 py-2 text-sm text-white whitespace-pre-wrap">
                  {m.content}
                </div>
              </div>
            ) : m.error ? (
              <div key={idx} className="rounded-lg border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/20 p-3 text-sm">
                <div className="text-red-600 dark:text-red-400">{m.content}</div>
                <button
                  onClick={() => handleAsk(lastQuestion)}
                  className="mt-2 rounded bg-red-100 dark:bg-red-900/40 px-2 py-1 text-xs text-red-700 dark:text-red-300 hover:bg-red-200 dark:hover:bg-red-900/60"
                >
                  {t("detail.exploreRetry")}
                </button>
              </div>
            ) : (
              <div key={idx} className="group rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-700/50 p-3">
                <div className="text-sm text-gray-800 dark:text-gray-100">
                  <MarkdownRenderer content={m.content} />
                </div>
                {m.sources && <SourceBadges sources={m.sources} />}
                <div className="mt-2 flex gap-3 text-xs text-gray-400 opacity-0 transition-opacity group-hover:opacity-100">
                  <button onClick={() => handleCopy(m.content)} className="hover:text-gray-600 dark:hover:text-gray-300">
                    <i className="fa-solid fa-copy mr-1" />
                    {t("detail.exploreCopy")}
                  </button>
                  <button onClick={() => handleSaveNote(idx)} className="hover:text-gray-600 dark:hover:text-gray-300">
                    <i className="fa-solid fa-bookmark mr-1" />
                    {t("detail.exploreSaveNote")}
                  </button>
                </div>
              </div>
            )
          )}
          {asking && (
            <div className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
              <i className="fa-solid fa-circle-notch fa-spin" />
              {t("detail.exploreThinking")}
            </div>
          )}
          <div ref={bottomRef} />
        </div>

        {/* 输入区 */}
        <div className="mt-3 flex items-end gap-2">
          <textarea
            ref={inputRef}
            value={input}
            onChange={(e) => {
              setInput(e.target.value);
              autoResize(e.target);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleAsk();
              }
            }}
            placeholder={t("detail.explorePlaceholder")}
            disabled={asking}
            rows={1}
            className="flex-1 resize-none rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-800 dark:text-gray-100 focus:border-blue-500 focus:outline-none disabled:opacity-50"
          />
          <button
            onClick={() => handleAsk()}
            disabled={asking || !input.trim()}
            title={projectName}
            className="rounded-lg bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
          >
            <i className="fa-solid fa-paper-plane" />
          </button>
        </div>
      </div>
    </div>
  );
}
