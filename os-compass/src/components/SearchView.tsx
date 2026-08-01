import { useState, useEffect, useCallback, useRef, startTransition } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { intentSearch, getSearchHistory, analyzeIntent } from '../api/search';
import { getRecentProjects } from '../api';
import { useToastStore } from '../stores/toastStore';
import type { SearchResult, SearchHistoryItem, ProjectMatch, IntentAnalysis } from '../api/search';
import type { Project } from '../types';

type SearchPhase = 'idle' | 'analyzing' | 'searching';

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

  const handleSubmit = async () => {
    if (!query.trim() || isLoading) return;

    console.log('[Search] ====== 开始搜索 ======');
    console.log('[Search] 查询内容:', query);

    const userMessage: MessageItem = {
      id: Date.now().toString(),
      role: 'user',
      content: query,
    };

    const loadingId = (Date.now() + 1).toString();
    loadingIdRef.current = loadingId;

    const loadingMessage: MessageItem = {
      id: loadingId,
      role: 'assistant',
      content: '正在分析语义...',
      phase: 'analyzing',
    };

    console.log('[Search] 添加用户消息到状态');
    setMessages(prev => [...prev, userMessage, loadingMessage]);
    setQuery('');
    setIsLoading(true);
    setSearchPhase('analyzing');

    function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
      console.log(`[Search] withTimeout: 设置 ${timeoutMs}ms 超时`);
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
          console.log('[Search] ⏰ withTimeout: 请求超时!');
          reject(new Error('timeout'));
        }, timeoutMs);
        promise
          .then(result => {
            clearTimeout(timer);
            console.log('[Search] ✅ withTimeout: 请求成功返回');
            resolve(result);
          })
          .catch(err => {
            clearTimeout(timer);
            console.log('[Search] ❌ withTimeout: 请求失败:', err?.message || err);
            reject(err);
          });
      });
    }

    const updatePhase = (phase: SearchPhase, content: string) => {
      console.log('[Search] updatePhase:', phase, '-', content);
      startTransition(() => {
        setSearchPhase(phase);
        if (loadingIdRef.current) {
          setMessages(prev => prev.map(msg =>
            msg.id === loadingIdRef.current
              ? { ...msg, phase, content }
              : msg
          ));
        }
      });
    };

    try {
      const cacheKey = generateCacheKey(query);
      console.log('[Search] 缓存Key:', cacheKey);
      
      let intent = getIntentFromCache(cacheKey);
      console.log('[Search] 缓存命中?', !!intent);

      if (!intent) {
        console.log('[Search] 缓存未命中，调用 analyzeIntent API...');
        try {
          intent = await withTimeout(analyzeIntent(query), 30000);
          console.log('[Search] analyzeIntent 返回:', JSON.stringify(intent));
          setIntentToCache(cacheKey, intent);
        } catch (e: unknown) {
          console.log('[Search] analyzeIntent 异常:', e);
          console.log('[Search] 降级为直接搜索');
          intent = { intent: 'clear', keywords: [query] };
        }
      }

      console.log('[Search] 最终意图:', JSON.stringify(intent));

      if (intent.intent === 'unclear' && intent.questions) {
        console.log('[Search] 意图不明确，显示追问');
        updatePhase('idle', intent.questions.join('\n'));
        if (loadingIdRef.current) {
          setMessages(prev => prev.map(msg =>
            msg.id === loadingIdRef.current
              ? { ...msg, clarification: intent }
              : msg
          ));
        }
      } else {
        const searchQuery = intent.keywords?.join(' ') || query;
        console.log('[Search] 最终搜索词:', searchQuery);
        
        const searchCacheKey = generateCacheKey(searchQuery);
        let result = getSearchFromCache(searchCacheKey);
        console.log('[Search] 搜索缓存命中?', !!result);

        if (!result) {
          try {
            updatePhase('searching', '正在搜索本地项目和联网查询...');
            console.log('[Search] 调用 intentSearch API...');
            const currentConvId = conversationId || undefined;
            result = await withTimeout(intentSearch(searchQuery, currentConvId), 60000);
            console.log('[Search] intentSearch 返回结果数:', result?.total);
            setSearchToCache(searchCacheKey, result);
          } catch (e: unknown) {
            console.log('[Search] intentSearch 异常:', e);
            console.log('[Search] 降级为空结果');
            result = { query: searchQuery, local_results: [], web_results: [], total: 0, conversation_id: '' };
          }
        }

        if (!conversationId && result.conversation_id) {
          setConversationId(result.conversation_id);
        }

        console.log('[Search] 更新消息显示结果');
        updatePhase('idle', `根据您的需求，我找到了 ${result.total} 个相关项目。`);
        if (loadingIdRef.current) {
          setMessages(prev => prev.map(msg =>
            msg.id === loadingIdRef.current
              ? { ...msg, content: `根据您的需求，我找到了 ${result.total} 个相关项目。`, results: result }
              : msg
          ));
        }
        loadHistory();
      }
    } catch (e: unknown) {
      console.log('[Search] 最终异常:', e);
      updatePhase('idle', '抱歉，搜索失败了，请稍后重试。');
      showToast(t('search.error.searchFailed') || '搜索失败', 'error');
    } finally {
      console.log('[Search] ====== 搜索完成 ======');
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
    setTimeout(handleSubmit, 0);
  };

  const handleProjectClick = async (projectUrl: string) => {
    try {
      await invoke('open_project_detail', { url: projectUrl });
    } catch {
      showToast(t('search.error.openFailed') || '打开项目详情失败', 'error');
    }
  };

  const getSourceIcon = (source: string) => {
    switch (source) {
      case 'local':
        return { icon: 'fa-database', color: 'text-blue-500' };
      case 'llm':
        return { icon: 'fa-wand-magic-sparkles', color: 'text-purple-500' };
      default:
        return { icon: 'globe', color: 'text-gray-400' };
    }
  };

  const getHealthColor = (score?: number) => {
    if (!score) return 'text-gray-400';
    if (score >= 80) return 'text-green-500';
    if (score >= 60) return 'text-yellow-500';
    return 'text-red-500';
  };

  const ProjectCard = (props: { item: ProjectMatch }) => {
    const { item } = props;
    const source = getSourceIcon(item.source);
    const healthColor = getHealthColor(item.health_score);

    return (
      <div
        className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl p-4 hover:border-blue-500 cursor-pointer transition"
        onClick={() => item.url && handleProjectClick(item.url)}
      >
        <div className="flex items-start gap-3">
          <div className="w-11 h-11 bg-gray-100 dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded-xl flex items-center justify-center flex-shrink-0">
            <i className={`fa-solid ${source.icon} ${source.color} text-xl`}></i>
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 flex-wrap mb-1">
              <h4 className="font-semibold text-gray-900 dark:text-white">{item.name}</h4>
              <span className={`flex items-center gap-1 px-2 py-0.5 rounded text-xs ${
                item.source === 'local'
                  ? 'bg-blue-100 dark:bg-blue-900/40 text-blue-600 dark:text-blue-400 border border-blue-200 dark:border-blue-700'
                  : 'bg-purple-100 dark:bg-purple-900/40 text-purple-600 dark:text-purple-400 border border-purple-200 dark:border-purple-700'
              }`}>
                <i className={`fa-solid ${source.icon} text-[10px]`}></i>
                {item.source === 'local' ? t('search.source.local') : t('search.source.llm')}
              </span>
              {item.health_score && (
                <span className={`flex items-center gap-1 px-2 py-0.5 rounded text-xs bg-red-100 dark:bg-red-900/30 border border-red-200 dark:border-red-700 ${healthColor}`}>
                  <i className="fa-solid fa-heart text-[10px]"></i>
                  {item.health_score}
                </span>
              )}
            </div>
            <p className="text-sm text-gray-500 dark:text-gray-400 line-clamp-2">{item.description || ''}</p>
            <div className="flex items-center gap-3 mt-2 text-xs text-gray-400 dark:text-gray-500">
              {item.stars && (
                <span className="flex items-center gap-1">
                  <i className="fa-solid fa-star text-yellow-500"></i>
                  {item.stars >= 1000 ? `${(item.stars / 1000).toFixed(0)}k` : item.stars}
                </span>
              )}
              {item.language && (
                <span className="flex items-center gap-1">
                  <i className="fa-solid fa-code"></i>
                  {item.language}
                </span>
              )}
            </div>
          </div>
          <div className="flex-shrink-0">
            <span className="flex items-center gap-1 px-3 py-1.5 bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400 rounded-lg text-xs">
              <i className="fa-solid fa-check"></i>
              {t('search.inLibrary') || '已入库'}
            </span>
          </div>
        </div>
      </div>
    );
  };

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
                        setTimeout(handleSubmit, 0);
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
                  {msg.phase ? (
                    <TypingIndicator phase={msg.phase} />
                  ) : (
                    <>
                      <p className={msg.role === 'user' ? '' : 'text-gray-700 dark:text-gray-300 leading-relaxed'}>
                        {msg.content}
                      </p>
                      {msg.results && (
                        <>
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
                          <div className="space-y-3">
                            {[...(msg.results.local_results || []), ...(msg.results.web_results || [])].map((item, idx) => (
                              <ProjectCard key={idx} item={item} />
                            ))}
                          </div>
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
                                          setTimeout(handleSubmit, 0);
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
                      {msg.clarification && (
                        <div className="mt-4 space-y-3">
                          <p className="text-sm text-gray-600 dark:text-gray-400">{t('search.clarification.prompt') || '请选择或补充您的需求：'}</p>
                          <div className="flex flex-wrap gap-2">
                            {msg.clarification.options?.map((option, idx) => (
                              <button
                                key={idx}
                                onClick={() => {
                                  setQuery(option);
                                  setTimeout(handleSubmit, 0);
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
                              if (e.key === 'Enter' && (e.target as HTMLInputElement).value.trim()) {
                                setQuery((e.target as HTMLInputElement).value);
                                setTimeout(handleSubmit, 0);
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
                onClick={handleSubmit}
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
                <button
                  key={item.id}
                  onClick={() => handleHistoryClick(item.query)}
                  className="w-full text-left px-3 py-2.5 text-sm rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-300 transition flex items-center gap-2 truncate"
                >
                  <i className="fa-regular fa-message text-gray-400 text-xs"></i>
                  <span className="truncate">{item.query}</span>
                </button>
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
                  onClick={() => project.url && handleProjectClick(project.url)}
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
