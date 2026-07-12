import { useState, useEffect, useCallback, useRef, KeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { intentSearch, getSearchHistory, clearSearchHistory, importSearchResult, SearchResult, SearchHistoryItem } from '../api/search';
import { useToastStore } from '../stores/toastStore';

export function SearchView() {
  const { t } = useTranslation();
  const { showToast } = useToastStore();
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult | null>(null);
  const [history, setHistory] = useState<SearchHistoryItem[]>([]);
  const [loading, setLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const loadHistory = useCallback(async () => {
    try {
      const data = await getSearchHistory(20);
      setHistory(data);
    } catch {
      showToast(t('search.error.loadHistoryFailed'), 'error');
    }
  }, [t, showToast]);

  useEffect(() => {
    loadHistory();
    inputRef.current?.focus();
  }, [loadHistory]);

  const handleSearch = async (searchQuery: string) => {
    if (!searchQuery.trim()) return;
    setLoading(true);
    try {
      const data = await intentSearch(searchQuery);
      setResults(data);
      loadHistory();
    } catch {
      showToast(t('search.error.searchFailed'), 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleSubmit = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      handleSearch(query);
    }
  };

  const handleClearHistory = async () => {
    try {
      await clearSearchHistory();
      setHistory([]);
      showToast(t('search.historyCleared'), 'success');
    } catch {
      showToast(t('search.error.clearFailed'), 'error');
    }
  };

  const handleImport = async (projectUrl: string, projectName: string) => {
    try {
      await importSearchResult(projectUrl, projectName);
      showToast(t('search.importSuccess'), 'success');
    } catch {
      showToast(t('search.error.importFailed'), 'error');
    }
  };

  const sourceColors: Record<string, string> = {
    local: 'bg-blue-600',
    cache: 'bg-green-600',
    web: 'bg-purple-600',
  };

  const sourceLabels: Record<string, string> = {
    local: t('search.source.local'),
    cache: t('search.source.cache'),
    web: t('search.source.web'),
  };

  return (
    <div className="flex h-full bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100">
      <main className="flex-1 flex flex-col">
        <header className="h-16 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 flex items-center px-6 flex-shrink-0">
          <div className="relative flex-1 max-w-2xl mx-auto">
            <i className="fa-solid fa-magnifying-glass absolute left-4 top-1/2 -translate-y-1/2 text-gray-400" />
            <input
              ref={inputRef}
              type="text"
              value={query}
              onChange={e => setQuery(e.target.value)}
              onKeyDown={handleSubmit}
              placeholder={t('search.placeholder')}
              className="w-full pl-12 pr-4 py-3 bg-gray-100 dark:bg-gray-700 rounded-xl text-gray-900 dark:text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
        </header>

        <div className="flex-1 overflow-auto p-6">
          {!results && !loading && (
            <div className="flex flex-col items-center justify-center h-full text-gray-400 dark:text-gray-500">
              <i className="fa-solid fa-robot text-5xl mb-4 text-gray-300 dark:text-gray-600" />
              <p className="text-lg mb-2">{t('search.welcome')}</p>
              <p className="text-sm">{t('search.hint')}</p>
            </div>
          )}

          {loading && (
            <div className="flex flex-col items-center justify-center h-full">
              <i className="fa-solid fa-spinner animate-spin text-3xl text-blue-500 mb-4" />
              <p className="text-gray-500 dark:text-gray-400">{t('search.searching')}</p>
            </div>
          )}

          {results && !loading && (
            <div className="max-w-3xl mx-auto">
              <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">
                {t('search.resultsCount', { count: results.total })}
              </p>
              <div className="space-y-4">
                {results.results.map((result, idx) => (
                  <div key={idx} className="bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700">
                    <div className="flex items-start gap-3">
                      <div className={`w-8 h-8 rounded-lg flex items-center justify-center text-white text-xs font-medium ${sourceColors[result.source] || 'bg-gray-600'}`}>
                        {result.source === 'local' && 'L'}
                        {result.source === 'cache' && 'C'}
                        {result.source === 'web' && 'W'}
                      </div>
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-1">
                          <span className={`text-xs px-2 py-0.5 rounded ${
                            result.source === 'local' ? 'bg-blue-100 dark:bg-blue-900/40 text-blue-600 dark:text-blue-400' :
                            result.source === 'cache' ? 'bg-green-100 dark:bg-green-900/40 text-green-600 dark:text-green-400' :
                            'bg-purple-100 dark:bg-purple-900/40 text-purple-600 dark:text-purple-400'
                          }`}>
                            {sourceLabels[result.source] || result.source}
                          </span>
                          {result.match_score > 0 && (
                            <span className="text-xs text-gray-400 dark:text-gray-500">
                              {t('search.matchScore', { score: Math.round(result.match_score * 100) })}
                            </span>
                          )}
                        </div>
                        <h3 className="font-medium mb-1">{result.project_name || t('search.unknownProject')}</h3>
                        {result.project_url && (
                          <a
                            href={result.project_url}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-sm text-blue-600 dark:text-blue-400 hover:underline"
                          >
                            {result.project_url}
                          </a>
                        )}
                        {result.description && (
                          <p className="text-sm text-gray-500 dark:text-gray-400 mt-2 line-clamp-2">
                            {result.description}
                          </p>
                        )}
                        {result.language && (
                          <span className="inline-block mt-2 px-2 py-0.5 text-xs bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 rounded">
                            {result.language}
                          </span>
                        )}
                        {result.project_url && (
                          <div className="flex items-center gap-2 mt-3">
                            <a
                              href={result.project_url}
                              target="_blank"
                              rel="noopener noreferrer"
                              className="px-3 py-1.5 text-xs bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded-lg transition"
                            >
                              <i className="fa-solid fa-external-link-alt mr-1" />
                              {t('search.viewOnGitHub')}
                            </a>
                            <button
                              onClick={() => handleImport(result.project_url!, result.project_name || 'Unknown')}
                              className="px-3 py-1.5 text-xs bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition"
                            >
                              <i className="fa-solid fa-download mr-1" />
                              {t('search.import')}
                            </button>
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </main>

      <aside className="w-72 bg-gray-50 dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700 p-4 overflow-auto">
        <div className="flex items-center justify-between mb-4">
          <h2 className="font-medium text-sm">{t('search.history')}</h2>
          {history.length > 0 && (
            <button
              onClick={handleClearHistory}
              className="text-xs text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
            >
              {t('search.clearHistory')}
            </button>
          )}
        </div>

        {history.length === 0 ? (
          <p className="text-gray-400 dark:text-gray-500 text-sm text-center py-8">
            {t('search.noHistory')}
          </p>
        ) : (
          <div className="space-y-1">
            {history.map(item => (
              <button
                key={item.id}
                onClick={() => handleSearch(item.query)}
                className="w-full text-left px-3 py-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition text-sm text-gray-600 dark:text-gray-300 truncate"
              >
                <i className="fa-regular fa-clock mr-2 text-gray-400" />
                {item.query}
              </button>
            ))}
          </div>
        )}
      </aside>
    </div>
  );
}
