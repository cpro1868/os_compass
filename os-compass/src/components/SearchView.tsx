import { useState, useEffect, useCallback, useRef, startTransition } from 'react';
import { useTranslation } from 'react-i18next';
import { openUrl } from '@tauri-apps/plugin-opener';
import {
  intentSearch,
  getSearchHistory,
  analyzeIntent,
  deleteSearchHistoryItem,
  recommendMoreByLLM,
} from '../api/search';
import { getRecentProjects } from '../api';
import { MarkdownRenderer } from './MarkdownRenderer';
import { useToastStore } from '../stores/toastStore';
import type { SearchResult, SearchHistoryItem, ProjectMatch, IntentAnalysis } from '../api/search';
import type { Project } from '../types';

type SearchPhase = 'idle' | 'analyzing' | 'searching';

export function shouldShowTypingIndicator(phase?: SearchPhase): boolean {
  return phase === 'analyzing' || phase === 'searching';
}

const STATUS_CONFIG: Record<string, { emoji: string; label: string }> = {
  TO_EXPLORE: { emoji: "💡", label: "待探索" },
  DIVING: { emoji: "🔬", label: "深度研究中" },
  IN_USE: { emoji: "✅", label: "已落地" },
  ABANDONED: { emoji: "🗑️", label: "弃用/避坑" },
};

interface MessageItem {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  results?: SearchResult;
  clarification?: IntentAnalysis;
  phase?: SearchPhase;
  recommendResults?: ProjectMatch[];
  rawText?: string;
}

export function SearchView() {
  const { t } = useTranslation();
  const { showToast } = useToastStore();
  const [messages, setMessages] = useState<MessageItem[]>([]);
  const [query, setQuery] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [, setSearchPhase] = useState<SearchPhase>('idle');
  const [conversationId, setConversationId] = useState<string | null>(null);
  const [history, setHistory] = useState<SearchHistoryItem[]>([]);
  const [expandedMessages, setExpandedMessages] = useState<Record<string, boolean>>({});
  const [recentProjects, setRecentProjects] = useState<Project[]>([]);
  const chatContainerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const intentCache = useRef<Map<string, { data: IntentAnalysis; expiresAt: number }>>(new Map());
  const searchCache = useRef<Map<string, { data: SearchResult; expiresAt: number }>>(new Map());
  const loadingIdRef = useRef<string | null>(null);

  const generateCacheKey = (text: string): string => {
    const normalized = text.toLowerCase().trim();
    let hash = 0;
    for (let i = 0; i < normalized.length; i++) {
      const char = normalized.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash;
    }
    return hash.toString(16);
  };

  const getIntentFromCache = (key: string): IntentAnalysis | null => {
    const cached = intentCache.current.get(key);
    if (cached && cached.expiresAt > Date.now()) {
      return cached.data;
    }
    if (cached) {
      intentCache.current.delete(key);
    }
    return null;
  };

  const setIntentToCache = (key: string, data: IntentAnalysis): void => {
    intentCache.current.set(key, {
      data,
      expiresAt: Date.now() + 2 * 60 * 60 * 1000,
    });
  };

  const getSearchFromCache = (key: string): SearchResult | null => {
    const cached = searchCache.current.get(key);
    if (cached && cached.expiresAt > Date.now()) {
      return cached.data;
    }
    if (cached) {
      searchCache.current.delete(key);
    }
    return null;
  };

  const setSearchToCache = (key: string, data: SearchResult): void => {
    searchCache.current.set(key, {
      data,
      expiresAt: Date.now() + 30 * 60 * 1000,
    });
  };

  const loadHistory = useCallback(async () => {
    try {
      const data = await getSearchHistory(20);
      setHistory(data);
    } catch (e) {
      console.error('Failed to load history:', e);
    }
  }, []);

  const loadRecentProjects = useCallback(async () => {
    try {
      const projects = await getRecentProjects(5);
      setRecentProjects(projects || []);
    } catch (e) {
      console.error('Failed to load recent projects:', e);
    }
  }, []);

  useEffect(() => {
    loadHistory();
    loadRecentProjects();
    inputRef.current?.focus();
  }, [loadHistory, loadRecentProjects]);

  useEffect(() => {
    if (chatContainerRef.current) {
      chatContainerRef.current.scrollTop = chatContainerRef.current.scrollHeight;
    }
  }, [messages]);

  useEffect(() => {
    const handleVaultChanged = () => {
      setMessages([]);
      setConversationId(null);
      intentCache.current.clear();
      searchCache.current.clear();
      loadHistory();
      loadRecentProjects();
    };
    window.addEventListener('vault-changed', handleVaultChanged);
    return () => window.removeEventListener('vault-changed', handleVaultChanged);
  }, [loadHistory, loadRecentProjects]);

  const handleSubmit = async (overrideQuery?: string) => {
    const textToSubmit = typeof overrideQuery === 'string' ? overrideQuery : query;
    if (!textToSubmit.trim() || isLoading) return;

    const userMessage: MessageItem = {
      id: Date.now().toString(),
      role: 'user',
      content: textToSubmit,
    };

    const loadingId = (Date.now() + 1).toString();
    loadingIdRef.current = loadingId;

    const loadingMessage: MessageItem = {
      id: loadingId,
      role: 'assistant',
      content: '正在分析语义...',
      phase: 'analyzing',
    };

    setMessages(prev => [...prev, userMessage, loadingMessage]);
    setQuery('');
    setIsLoading(true);
    setSearchPhase('analyzing');

    function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error('timeout')), timeoutMs);
        promise
          .then(result => { clearTimeout(timer); resolve(result); })
          .catch(err => { clearTimeout(timer); reject(err); });
      });
    }

    const updatePhase = (phase: SearchPhase, content: string) => {
      startTransition(() => {
        setSearchPhase(phase);
        setMessages(prev => prev.map(msg =>
          msg.id === loadingId ? { ...msg, phase, content } : msg
        ));
      });
    };

    try {
      const historyContext: Array<{ role: 'user' | 'assistant'; content: string }> = messages
        .filter((msg) => msg.id !== loadingId && (msg.phase === 'idle' || !msg.phase))
        .slice(-6)
        .map((msg) => {
          let text = msg.content || '';
          if (msg.clarification?.questions) {
            text = msg.clarification.questions.join(' ');
          } else if (msg.rawText) {
            text = msg.rawText;
          }
          return {
            role: msg.role,
            content: text.trim(),
          };
        })
        .filter((it) => it.content.length > 0);

      const contextDigest = historyContext.map((it) => `${it.role}:${it.content}`).join('|');
      const cacheKey = generateCacheKey(`${contextDigest}#${textToSubmit}`);
      let intent = getIntentFromCache(cacheKey);

      if (!intent) {
        try {
          updatePhase('analyzing', '正在分析语义...');
          intent = await withTimeout(analyzeIntent(textToSubmit, historyContext), 30000);
          setIntentToCache(cacheKey, intent);
        } catch {
          intent = { intent: 'clear', keywords: [textToSubmit] };
        }
      }

      if (intent.intent === 'unclear' && intent.questions) {
        updatePhase('idle', intent.questions.join('\n'));
        setMessages(prev => prev.map(msg =>
          msg.id === loadingId ? { ...msg, clarification: intent } : msg
        ));
      } else {
        const searchQuery = intent.keywords?.join(' ') || textToSubmit;
        const searchCacheKey = generateCacheKey(searchQuery);
        let result = getSearchFromCache(searchCacheKey);

        if (!result) {
          try {
            updatePhase('searching', '正在搜索本地项目和联网查询...');
            const currentConvId = conversationId || undefined;
            result = await withTimeout(intentSearch(searchQuery, currentConvId), 60000);
            setSearchToCache(searchCacheKey, result);
          } catch {
            result = { query: searchQuery, local_results: [], web_results: [], total: 0, conversation_id: '' };
          }
        }

        if (!conversationId && result.conversation_id) {
          setConversationId(result.conversation_id);
        }

        updatePhase('idle', `根据您的需求，我找到了 ${result.total} 个相关项目。`);
        setMessages(prev => prev.map(msg =>
          msg.id === loadingId
            ? { ...msg, content: `根据您的需求，我找到了 ${result.total} 个相关项目。`, results: result }
            : msg
        ));
        loadHistory();
      }
    } catch {
      updatePhase('idle', '抱歉，搜索失败了，请稍后重试。');
      showToast(t('search.error.searchFailed') || '搜索失败', 'error');
    } finally {
      setIsLoading(false);
      setSearchPhase('idle');
      loadingIdRef.current = null;
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  const handleClear = () => {
    setMessages([]);
    setConversationId(null);
    loadHistory();
  };

  const handleHistoryClick = (searchQuery: string) => {
    setQuery(searchQuery);
    handleSubmit(searchQuery);
  };

  const handleDeleteHistoryItem = async (id: number) => {
    try {
      await deleteSearchHistoryItem(id);
      setHistory((prev) => prev.filter((item) => item.id !== id));
    } catch (e) {
      console.error('Failed to delete history item:', e);
      showToast(t('search.historyDeleteFailed') || '删除失败', 'error');
    }
  };

  const askMoreForQuery = async (messageId: string, baseQuery: string): Promise<void> => {
    const newId = `${messageId}-more-${Date.now()}`;
    const loadingText = t('search.recommendLoading') || '正在让大模型推荐，约需 20~30 秒，请稍候...';
    setMessages((prev) => [
      ...prev,
      { id: newId, role: 'assistant', content: loadingText },
    ]);
    try {
      const res = await recommendMoreByLLM(baseQuery, 5);
      const text = res?.raw_text?.trim();
      if (text) {
        setMessages((prev) =>
          prev.map((msg) =>
            msg.id === newId
              ? {
                  ...msg,
                  phase: 'idle',
                  content: '',
                  rawText: text,
                }
              : msg
          )
        );
        return;
      }
      const items: ProjectMatch[] = (res?.items || []).map((it) => ({
        ...it,
        source: 'llm' as const,
      }));
      if (items.length === 0) {
        setMessages((prev) =>
          prev.map((msg) =>
            msg.id === newId
              ? { ...msg, phase: 'idle', content: t('search.recommendEmpty') || '暂无更多推荐' }
              : msg
          )
        );
        return;
      }
      setMessages((prev) =>
        prev.map((msg) =>
          msg.id === newId
            ? {
                ...msg,
                phase: 'idle',
                content: t('search.recommendTitle') || '为你补充推荐以下项目：',
                recommendResults: items,
              }
            : msg
        )
      );
    } catch (e) {
      console.error('recommendMore failed:', e);
      setMessages((prev) =>
        prev.map((msg) =>
          msg.id === newId
            ? { ...msg, phase: 'idle', content: t('search.recommendFailed') || '推荐失败，请稍后重试' }
            : msg
        )
      );
    }
  };

  const handleProjectItemClick = async (item: ProjectMatch) => {
    if (item.source === 'local' && item.project_id) {
      window.dispatchEvent(
        new CustomEvent('openProjectDetail', {
          detail: { projectId: item.project_id },
        })
      );
      return;
    }

    if (!item.url) {
      showToast(t('search.error.openFailed') || '打开项目详情失败', 'error');
      return;
    }

    let target = item.url.trim();
    if (!target) {
      showToast(t('search.error.openFailed') || '打开项目详情失败', 'error');
      return;
    }
    if (!/^https?:\/\//i.test(target)) {
      target = `https://${target}`;
    }

    try {
      await openUrl(target);
    } catch (e) {
      console.error('openUrl failed:', e);
      showToast(t('search.error.openFailed') || '打开项目详情失败', 'error');
    }
  };

function normalizeLanguage(lang?: string | null): string | null {
  if (!lang) return null;
  const trimmed = lang.trim();
  if (!trimmed) return null;
  if (trimmed.startsWith('[')) {
    try {
      const parsed = JSON.parse(trimmed);
      if (Array.isArray(parsed)) {
        const first = parsed.find((x) => typeof x === 'string' && x.trim());
        return first ? first.trim() : null;
      }
    } catch {
      // ignore
    }
  }
  return trimmed;
}

interface ProjectCardProps {
  item: ProjectMatch;
  onClick: (item: ProjectMatch) => void;
}

function ProjectCard({ item, onClick }: ProjectCardProps) {
  const lang = normalizeLanguage(item.language);
  return (
    <div
      className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl p-4 hover:border-blue-500 cursor-pointer transition"
      onClick={() => onClick(item)}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex-1 min-w-0">
          <h4 className="font-semibold text-gray-900 dark:text-white text-base leading-snug truncate">
            {item.name}
          </h4>
          {item.description ? (
            <p className="text-sm text-gray-600 dark:text-gray-300 line-clamp-2 mt-1 leading-relaxed">
              {item.description}
            </p>
          ) : null}
        </div>
        {lang ? (
          <span className="flex-shrink-0 text-xs px-2.5 py-1 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-md font-medium border border-gray-200 dark:border-gray-600">
            {lang}
          </span>
        ) : null}
      </div>
    </div>
  );
}

interface ResultsListProps {
  local: ProjectMatch[];
  web: ProjectMatch[];
  query: string;
  messageId: string;
  expanded?: boolean;
  onToggleExpand?: (expanded: boolean) => void;
  onAskMore: (messageId: string, query: string) => Promise<void>;
  onItemClick: (item: ProjectMatch) => void;
}

function ResultsList(props: ResultsListProps) {
  const { local, web, query, messageId, expanded: controlledExpanded, onToggleExpand, onAskMore, onItemClick } = props;
  const { t } = useTranslation();
  const [internalExpanded, setInternalExpanded] = useState(false);
  const [asking, setAsking] = useState(false);
  const expanded = controlledExpanded !== undefined ? controlledExpanded : internalExpanded;
  const setExpanded = (val: boolean | ((prev: boolean) => boolean)) => {
    const next = typeof val === 'function' ? val(expanded) : val;
    setInternalExpanded(next);
    onToggleExpand?.(next);
  };
  const all = [...local, ...web];
  const COLLAPSE_THRESHOLD = 5;
  const shouldCollapse = all.length > COLLAPSE_THRESHOLD;
  const visible = shouldCollapse && !expanded ? all.slice(0, COLLAPSE_THRESHOLD) : all;

  return (
    <div className="space-y-3">
      {visible.map((item, idx) => (
        <ProjectCard key={`${item.url}-${item.project_id || idx}`} item={item} onClick={onItemClick} />
      ))}
      {shouldCollapse && (
        <div className="flex justify-center pt-1">
          <button
            type="button"
            onClick={() => setExpanded((v) => !v)}
            className="text-xs text-blue-500 hover:text-blue-400 transition flex items-center gap-1"
          >
            <i className={`fa-solid ${expanded ? 'fa-chevron-up' : 'fa-chevron-down'}`}></i>
            {expanded
              ? (t('search.collapse') || '收起')
              : (t('search.showAll', { count: all.length }) || `展开全部（共 ${all.length} 条）`)}
          </button>
        </div>
      )}
      <div className="flex justify-center pt-2">
        <button
          type="button"
          disabled={asking}
          onClick={async () => {
            setAsking(true);
            try {
              await onAskMore(messageId, query);
            } finally {
              setAsking(false);
            }
          }}
          className="text-xs px-3 py-1.5 bg-purple-100 dark:bg-purple-900/30 hover:bg-purple-200 dark:hover:bg-purple-900/50 disabled:opacity-50 text-purple-600 dark:text-purple-400 rounded-md border border-purple-200 dark:border-purple-800 transition flex items-center gap-1.5"
        >
          <i className={`fa-solid ${asking ? 'fa-spinner fa-spin' : 'fa-wand-magic-sparkles'}`}></i>
          {asking
            ? (t('search.recommending') || '正在让大模型推荐...')
            : (t('search.recommendMore') || '让大模型再推荐 5 个')}
        </button>
      </div>
    </div>
  );
}

  const TypingIndicator = (props: { phase?: SearchPhase }) => (
    <div className="flex items-center gap-3 text-gray-500 dark:text-gray-400 text-sm">
      <div className="flex gap-1">
        <span className="w-2 h-2 bg-blue-500 rounded-full animate-bounce"></span>
        <span className="w-2 h-2 bg-blue-500 rounded-full animate-bounce" style={{ animationDelay: '100ms' }}></span>
        <span className="w-2 h-2 bg-blue-500 rounded-full animate-bounce" style={{ animationDelay: '200ms' }}></span>
      </div>
      <span>{props.phase === 'analyzing' ? '正在分析语义...' : props.phase === 'searching' ? '正在搜索...' : t('search.searching') || '正在搜索...'}</span>
    </div>
  );

  return (
    <div className="flex h-full bg-white dark:bg-gray-900">
      <main className="flex-1 flex flex-col overflow-hidden">
        <header className="h-14 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between px-6 bg-white dark:bg-gray-800">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-gradient-to-br from-blue-500 to-indigo-600 rounded-lg flex items-center justify-center">
              <i className="fa-solid fa-compass text-white text-sm"></i>
            </div>
            <div>
              <h1 className="font-semibold text-sm text-gray-900 dark:text-white">{t('search.title') || '意图搜索'}</h1>
              <p className="text-xs text-gray-500 dark:text-gray-400">{t('search.subtitle') || '本地向量 + LLM 联网推荐'}</p>
            </div>
          </div>
          <button
            onClick={handleClear}
            className="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition"
            title={t('search.clear') || '清除对话'}
          >
            <i className="fa-solid fa-trash-can text-gray-400 text-sm"></i>
          </button>
        </header>

        <div ref={chatContainerRef} className="flex-1 overflow-auto p-6 space-y-6 bg-gray-50 dark:bg-gray-900">
          {messages.length === 0 && (
            <div className="flex gap-4 max-w-3xl mx-auto">
              <div className="w-10 h-10 bg-gradient-to-br from-blue-500 to-indigo-600 rounded-xl flex-shrink-0 flex items-center justify-center shadow-lg">
                <i className="fa-solid fa-compass text-white"></i>
              </div>
              <div className="flex-1">
                <div className="bg-white dark:bg-gray-800 rounded-2xl rounded-tl-sm p-5 border border-gray-200 dark:border-gray-700">
                  <p className="text-gray-700 dark:text-gray-300 leading-relaxed">
                    {t('search.welcome') || '您好！我是您的开源项目助手。请描述您的需求，我将通过'}
                    <strong className="text-blue-500 dark:text-blue-400">{t('search.vectorSearch') || '向量语义匹配'}</strong>
                    {t('search.welcomeMiddle') || '搜索您的本地项目库，并通过'}
                    <strong className="text-purple-500 dark:text-purple-400">{t('search.llmSearch') || 'LLM 联网搜索'}</strong>
                    {t('search.welcomeEnd') || '为您推荐相关开源项目。'}
                  </p>
                </div>
                <div className="mt-4 flex flex-wrap gap-2">
                  {['前端状态管理库', 'Python Web 框架', 'Rust 日志库', '开源人脸识别'].map((hint) => (
                    <button
                      key={hint}
                      onClick={() => {
                        setQuery(hint);
                        handleSubmit(hint);
                      }}
                      className="px-3 py-1.5 text-xs bg-white dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700 border border-gray-200 dark:border-gray-700 hover:border-blue-500 rounded-full transition flex items-center gap-1.5 text-gray-700 dark:text-gray-300"
                    >
                      <i className="fa-solid fa-bolt text-yellow-500"></i>
                      {hint}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          )}

          {messages.map((msg) => (
            <div key={msg.id} className="flex gap-4 max-w-3xl mx-auto">
              <div className={`w-10 h-10 rounded-xl flex-shrink-0 flex items-center justify-center ${
                msg.role === 'user'
                  ? 'bg-gray-200 dark:bg-gray-700'
                  : 'bg-gradient-to-br from-blue-500 to-indigo-600 shadow-lg'
              }`}>
                {msg.role === 'user'
                  ? <i className="fa-solid fa-user text-gray-600 dark:text-gray-300"></i>
                  : <i className="fa-solid fa-compass text-white"></i>}
              </div>
              <div className="flex-1">
                <div className={
                  msg.role === 'user'
                    ? 'bg-blue-600 text-white rounded-2xl rounded-tr-sm p-4'
                    : 'bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-2xl rounded-tl-sm p-4'
                }>
                  {shouldShowTypingIndicator(msg.phase) ? (
                    <TypingIndicator phase={msg.phase} />
                  ) : (
                    <>
                      {msg.rawText ? (
                        <div className="text-sm text-gray-700 dark:text-gray-300">
                          <MarkdownRenderer content={msg.rawText} className="prose prose-sm dark:prose-invert max-w-none" />
                        </div>
                      ) : (
                        <p className={msg.role === 'user' ? '' : 'text-gray-700 dark:text-gray-300 leading-relaxed'}>
                          {msg.content}
                        </p>
                      )}
                       {msg.results && (
                         <>
                           {msg.results.llm_text && (
                             <div className="mt-4 whitespace-pre-wrap text-sm text-gray-700 dark:text-gray-300 leading-relaxed">
                               {msg.results.llm_text}
                             </div>
                           )}
                           <div className="flex flex-wrap gap-3 my-4">
                            <div className="flex items-center gap-1.5 px-2.5 py-1 bg-blue-100 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-800 rounded-lg text-xs">
                              <i className="fa-solid fa-database text-blue-500"></i>
                              <span className="text-blue-600 dark:text-blue-400">
                                {t('search.localVector') || '本地向量'}: {msg.results.local_results?.length || 0}
                              </span>
                            </div>
                            <div className="flex items-center gap-1.5 px-2.5 py-1 bg-purple-100 dark:bg-purple-900/30 border border-purple-200 dark:border-purple-800 rounded-lg text-xs">
                              <i className="fa-solid fa-globe text-purple-500"></i>
                              <span className="text-purple-600 dark:text-purple-400">
                                {t('search.llmOnline') || 'LLM联网'}: {msg.results.web_results?.length || 0}
                              </span>
                            </div>
                          </div>
                          <ResultsList
                            local={msg.results.local_results || []}
                            web={msg.results.web_results || []}
                            query={msg.results.query}
                            messageId={msg.id}
                            expanded={expandedMessages[msg.id]}
                            onToggleExpand={(val) =>
                              setExpandedMessages((prev) => ({ ...prev, [msg.id]: val }))
                            }
                            onAskMore={askMoreForQuery}
                            onItemClick={handleProjectItemClick}
                          />
                          {msg.results.recommendation && (
                            <div className="mt-4 p-4 bg-gray-50 dark:bg-gray-700/50 rounded-xl border border-gray-200 dark:border-gray-700">
                              <h4 className="font-semibold text-sm text-gray-700 dark:text-gray-300 mb-3 flex items-center gap-2">
                                <i className="fa-solid fa-wand-magic-sparkles text-blue-500"></i>
                                {t('search.smartRecommendation') || '智能建议'}
                              </h4>
                              {msg.results.recommendation.categories.length > 0 && (
                                <div className="mb-3">
                                  <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">{t('search.relatedCategories') || '相关分类'}</p>
                                  <div className="flex flex-wrap gap-1.5">
                                    {msg.results.recommendation.categories.map((cat, idx) => (
                                      <span key={idx} className="px-2 py-0.5 bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 text-xs rounded-md">
                                        {cat}
                                      </span>
                                    ))}
                                  </div>
                                </div>
                              )}
                              {msg.results.recommendation.tags.length > 0 && (
                                <div className="mb-3">
                                  <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">{t('search.relatedTags') || '关联标签'}</p>
                                  <div className="flex flex-wrap gap-1.5">
                                    {msg.results.recommendation.tags.map((tag, idx) => (
                                      <span key={idx} className="px-2 py-0.5 bg-gray-200 dark:bg-gray-600 text-gray-600 dark:text-gray-300 text-xs rounded-md">
                                        {tag}
                                      </span>
                                    ))}
                                  </div>
                                </div>
                              )}
                              {msg.results.recommendation.suggestions.length > 0 && (
                                <div>
                                  <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">{t('search.expandedSearch') || '扩展搜索'}</p>
                                  <div className="flex flex-wrap gap-1.5">
                                    {msg.results.recommendation.suggestions.map((suggestion, idx) => (
                                      <button
                                        key={idx}
                                        onClick={() => {
                                          setQuery(suggestion);
                                          handleSubmit(suggestion);
                                        }}
                                        className="px-2 py-0.5 bg-purple-100 dark:bg-purple-900/30 text-purple-600 dark:text-purple-400 text-xs rounded-md hover:bg-purple-200 dark:hover:bg-purple-900/50 transition"
                                      >
                                        {suggestion}
                                      </button>
                                    ))}
                                  </div>
                                </div>
                              )}
                            </div>
                          )}
                        </>
                      )}
                      {msg.recommendResults && msg.recommendResults.length > 0 && (
                        <div className="mt-4 space-y-3">
                          {msg.recommendResults.map((item, idx) => (
                            <ProjectCard
                              key={`${item.url}-${idx}`}
                              item={item}
                              onClick={handleProjectItemClick}
                            />
                          ))}
                        </div>
                      )}
                      {msg.clarification && (
                        <div className="mt-4 space-y-3">
                          <p className="text-sm text-gray-600 dark:text-gray-400">{t('search.clarification.prompt') || '请选择或补充您的需求：'}</p>
                          <div className="flex flex-wrap gap-2">
                            {msg.clarification.options?.map((option, idx) => (
                              <button
                                key={idx}
                                onClick={() => {
                                  setQuery(option);
                                  handleSubmit(option);
                                }}
                                className="px-3 py-1.5 text-sm bg-gray-100 dark:bg-gray-700 hover:bg-blue-100 dark:hover:bg-blue-900/30 border border-gray-200 dark:border-gray-600 hover:border-blue-500 rounded-lg transition flex items-center gap-1.5"
                              >
                                <i className="fa-solid fa-hand-pointer text-blue-500"></i>
                                {option}
                              </button>
                            ))}
                          </div>
                          <input
                            type="text"
                            placeholder={t('search.clarification.placeholder') || '补充您的具体需求...'}
                            className="w-full px-3 py-2 text-sm bg-gray-50 dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded-lg focus:outline-none focus:border-blue-500"
                            onKeyDown={(e) => {
                              const val = (e.target as HTMLInputElement).value.trim();
                              if (e.key === 'Enter' && val) {
                                setQuery(val);
                                handleSubmit(val);
                              }
                            }}
                          />
                        </div>
                      )}
                    </>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>

        <div className="border-t border-gray-200 dark:border-gray-700 p-4 bg-white dark:bg-gray-800">
          <div className="max-w-3xl mx-auto">
            <div className="relative bg-gray-100 dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded-2xl shadow-sm focus-within:border-blue-500 focus-within:ring-2 focus-within:ring-blue-900/50 transition">
              <textarea
                ref={inputRef}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={handleKeyDown}
                rows={1}
                placeholder={t('search.placeholder') || '描述您的需求，例如：找一个人工智能相关的开源项目...'}
                className="w-full px-4 py-3 pr-14 text-gray-900 dark:text-white rounded-2xl resize-none focus:outline-none placeholder-gray-400 dark:placeholder-gray-500 bg-transparent"
              />
                <button
                  onClick={() => handleSubmit()}
                  disabled={!query.trim() || isLoading}
                className="absolute right-2 bottom-2 w-9 h-9 bg-gradient-to-r from-blue-500 to-indigo-600 hover:from-blue-400 hover:to-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-xl flex items-center justify-center transition shadow-lg"
              >
                <i className="fa-solid fa-paper-plane text-sm"></i>
              </button>
            </div>
            <div className="flex items-center justify-between mt-2 px-1">
              <p className="text-xs text-gray-500 dark:text-gray-400 flex items-center gap-1">
                <i className="fa-solid fa-wand-magic-sparkles text-blue-500 mr-1"></i>
                {t('search.aiTip') || 'AI 助手基于语义理解搜索'}
              </p>
              <p className="text-xs text-gray-500 dark:text-gray-400">
                <kbd className="px-1.5 py-0.5 bg-gray-200 dark:bg-gray-700 rounded text-gray-600 dark:text-gray-400">Enter</kbd> {t('search.send') || '发送'} · <kbd className="px-1.5 py-0.5 bg-gray-200 dark:bg-gray-700 rounded text-gray-600 dark:text-gray-400">Shift+Enter</kbd> {t('search.newline') || '换行'}
              </p>
            </div>
          </div>
        </div>
      </main>

      <aside className="w-72 bg-white dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700 flex flex-col">
        <div className="p-4 border-b border-gray-200 dark:border-gray-700">
          <h2 className="font-semibold text-sm text-gray-700 dark:text-gray-300 flex items-center gap-2">
            <i className="fa-solid fa-clock-rotate-left"></i>
            {t('search.history') || '搜索历史'}
          </h2>
        </div>
        <div className="flex-1 overflow-auto p-2">
          {history.length === 0 ? (
            <p className="text-sm text-gray-500 dark:text-gray-400 text-center py-4">{t('search.noHistory') || '暂无搜索记录'}</p>
          ) : (
            <div className="space-y-1">
              {history.map((item) => (
                <div
                  key={item.id}
                  className="group flex items-center gap-1 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition"
                >
                  <button
                    type="button"
                    onClick={() => handleHistoryClick(item.query)}
                    className="flex-1 min-w-0 text-left px-3 py-2.5 text-sm text-gray-700 dark:text-gray-300 truncate flex items-center gap-2"
                  >
                    <i className="fa-regular fa-message text-gray-400 text-xs"></i>
                    <span className="truncate">{item.query}</span>
                  </button>
                  <button
                    type="button"
                    aria-label={t('search.deleteHistoryItem') || '删除该条历史'}
                    title={t('search.deleteHistoryItem') || '删除该条历史'}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDeleteHistoryItem(item.id);
                    }}
                    className="opacity-0 group-hover:opacity-100 focus:opacity-100 p-1.5 mr-1 text-gray-400 hover:text-red-500 rounded transition"
                  >
                    <i className="fa-solid fa-xmark text-xs"></i>
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="p-4 border-t border-gray-200 dark:border-gray-700">
          <h2 className="font-semibold text-sm text-gray-700 dark:text-gray-300 flex items-center gap-2 mb-3">
            <i className="fa-solid fa-history"></i>
            {t('search.recentProjects') || '最近使用'}
          </h2>
          <div className="space-y-2">
            {recentProjects.length === 0 ? (
              <p className="text-xs text-gray-500 dark:text-gray-400">{t('search.noRecent') || '暂无最近项目'}</p>
            ) : (
              recentProjects.map((project) => (
                <div
                  key={project.id}
                  onClick={() =>
                    handleProjectItemClick({
                      name: project.name,
                      url: project.url || '',
                      description: project.description || undefined,
                      language: project.languages || undefined,
                      source: 'local',
                      match_score: 1.0,
                      project_id: project.id,
                    })
                  }
                  className="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-2.5 border border-gray-200 dark:border-gray-700 hover:border-blue-500 cursor-pointer transition"
                >
                  <div className="flex items-center gap-2">
                    <div className="w-7 h-7 bg-blue-100 dark:bg-blue-900/30 rounded-md flex items-center justify-center">
                      <i className="fa-solid fa-robot text-blue-500 text-xs"></i>
                    </div>
                    <div className="flex-1 min-w-0">
                      <h4 className="text-xs font-medium truncate text-gray-800 dark:text-gray-200">{project.name}</h4>
                      <p className="text-[10px] text-gray-500 dark:text-gray-400">
                        {STATUS_CONFIG[project.lifecycle_status]?.label || project.lifecycle_status} · {project.stars || 0} ⭐
                      </p>
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        <div className="p-3 border-t border-gray-200 dark:border-gray-700">
          <button className="w-full text-left px-3 py-2 text-sm rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-400 transition flex items-center gap-2">
            <i className="fa-solid fa-plus"></i>
            {t('search.newSearch') || '新建搜索'}
          </button>
        </div>
      </aside>
    </div>
  );
}
