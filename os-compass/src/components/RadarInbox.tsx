import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { listRadarSources, addRadarSource, updateRadarSource, deleteRadarSource, getRadarItems, triggerRadarScan, radarItemAction, clearRadarAll, getSupportedPlatformDomains, RadarSource, RadarItem, RadarSourceInput } from '../api/radar';
import { useToastStore } from '../stores/toastStore';

export function RadarInbox() {
  const { t } = useTranslation();
  const { showToast } = useToastStore();
  const [sources, setSources] = useState<RadarSource[]>([]);
  const [items, setItems] = useState<RadarItem[]>([]);
  const [activeTab, setActiveTab] = useState<string>('all');
  const [scanning, setScanning] = useState(false);
  const [showAddSource, setShowAddSource] = useState(false);
  const [newSource, setNewSource] = useState<RadarSourceInput>({
    name: '',
    url: '',
    sourceType: 'rss',
    platform: '',
    checkInterval: 3600,
    proxyEnabled: false,
    proxyProtocol: 'http',
    proxyHost: '',
    proxyPort: 0,
    proxyUsername: '',
    proxyPassword: '',
  });
  const [saving, setSaving] = useState(false);
  const [editingSource, setEditingSource] = useState<RadarSource | null>(null);
  const [showClearConfirm, setShowClearConfirm] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [timeRange, setTimeRange] = useState<string>('1d');
  const [importUrl, setImportUrl] = useState<string | null>(null);
  const [supportedDomains, setSupportedDomains] = useState<string[]>([]);
  // 搜索相关状态
  const [searchKeyword, setSearchKeyword] = useState('');
  const [searchSourceIds, setSearchSourceIds] = useState<string>('');
  const [searchStartDate, setSearchStartDate] = useState<string>('');
  const [searchEndDate, setSearchEndDate] = useState<string>('');
  const [pagination, setPagination] = useState({ page: 1, pageSize: 20, total: 0, totalPages: 0 });

  const loadSupportedDomains = useCallback(async () => {
    try {
      const domains = await getSupportedPlatformDomains();
      setSupportedDomains(domains);
    } catch (e) {
      console.error('Failed to load supported domains:', e);
    }
  }, []);

  const findSupportedUrls = (description: string | null): string[] => {
    if (!description || supportedDomains.length === 0) return [];
    const urlRegex = /https?:\/\/[^\s<>"']+/g;
    const matches = description.match(urlRegex) || [];
    return matches.filter(url => {
      const lower = url.toLowerCase();
      return supportedDomains.some(domain => lower.includes(domain.toLowerCase()));
    });
  };

  const handleImport = (urls: string[]) => {
    if (urls.length === 1) {
      setImportUrl(urls[0]);
    } else {
      const urlsText = urls.join('\n');
      setImportUrl(urlsText);
    }
  };

  const loadSources = useCallback(async () => {
    try {
      const data = await listRadarSources();
      setSources(data);
    } catch (e) {
      console.error('Failed to load sources:', e);
    }
  }, []);

  const loadItems = useCallback(async () => {
    try {
      const result = await getRadarItems(
        activeTab === 'all' ? undefined : activeTab,
        undefined,
        searchKeyword || undefined,
        searchSourceIds || undefined,
        searchStartDate || undefined,
        searchEndDate || undefined,
        pagination.page,
        pagination.pageSize
      );
      if (result === undefined || result === null) {
        console.error('[RadarInbox] getRadarItems returned null/undefined');
        showToast('加载数据失败：返回数据为空', 'error');
        return;
      }
      console.log('[RadarInbox] getRadarItems returned:', result.items.length, 'items (total:', result.total, ')');
      setItems(result.items);
      setPagination(prev => ({
        ...prev,
        total: result.total,
        totalPages: result.total_pages,
      }));
    } catch (e) {
      console.error('[RadarInbox] Failed to load items:', e);
      showToast('加载情报失败: ' + String(e), 'error');
    }
  }, [activeTab, searchKeyword, searchSourceIds, searchStartDate, searchEndDate, pagination.page, pagination.pageSize]);

  useEffect(() => {
    loadSources();
    loadItems();
    loadSupportedDomains();
    
    // 监听仓库切换事件，切换后重新加载数据
    const handleVaultChanged = () => {
      console.log('[RadarInbox] vault changed, reloading...');
      loadSources();
      loadItems();
      loadSupportedDomains();
    };
    window.addEventListener('vault-changed', handleVaultChanged);
    return () => window.removeEventListener('vault-changed', handleVaultChanged);
  }, [loadSources, loadItems, loadSupportedDomains]);

  const handleScan = async () => {
    setScanning(true);
    try {
      const result = await triggerRadarScan(undefined, timeRange);
      showToast(t('radar.scanComplete', { count: result.newItems }), 'success');
      loadItems();
      loadSources();
    } catch (e) {
      console.error('Scan failed:', e);
      showToast(t('radar.error.scanFailed'), 'error');
    } finally {
      setScanning(false);
    }
  };

  const handleEditSource = (source: RadarSource) => {
    setEditingSource(source);
    setNewSource({
      name: source.name,
      url: source.url,
      sourceType: source.source_type,
      platform: source.platform || '',
      checkInterval: source.check_interval,
      proxyEnabled: source.proxy_enabled,
      proxyProtocol: source.proxy_protocol,
      proxyHost: source.proxy_host || '',
      proxyPort: source.proxy_port,
      proxyUsername: source.proxy_username || '',
      proxyPassword: source.proxy_password || '',
    });
    setShowAddSource(true);
  };

  const handleDeleteSource = async (sourceId: number) => {
    if (!window.confirm(t('radar.confirmDelete'))) return;
    try {
      await deleteRadarSource(sourceId);
      showToast(t('radar.sourceDeleted'), 'success');
      loadSources();
    } catch {
      showToast(t('radar.error.deleteFailed'), 'error');
    }
  };

  const handleAction = async (itemId: number, action: 'collect' | 'blacklist') => {
    try {
      await radarItemAction(itemId, action);
      showToast(t(`radar.action.${action}Success`), 'success');
      loadItems();
    } catch {
      showToast(t('radar.error.actionFailed'), 'error');
    }
  };

  const handleClearAll = async () => {
    setClearing(true);
    try {
      await clearRadarAll();
      showToast(t('radar.clearedSuccess'), 'success');
      loadItems();
    } catch {
      showToast(t('radar.error.clearFailed'), 'error');
    } finally {
      setClearing(false);
      setShowClearConfirm(false);
    }
  };

  const highlightLinks = (text: string) => {
    const urlRegex = /(https?:\/\/[^\s<]+)/g;
    const parts = text.split(urlRegex);
    return parts.map((part, i) =>
      urlRegex.test(part) ? (
        <a key={i} href={part} target="_blank" rel="noopener noreferrer"
          className="text-blue-600 dark:text-blue-400 underline hover:text-blue-800 dark:hover:text-blue-300">
          {part}
        </a>
      ) : part
    );
  };

  const detectSourceType = (url: string): string => {
    const lower = url.toLowerCase();
    if (lower.includes('t.me/')) return 'telegram';
    if (lower.includes('.rss') || lower.includes('feed') || lower.includes('atom')) return 'rss';
    if (lower.includes('sitemap')) return 'rss';
    return 'web_crawl';
  };

  const validateSourceType = (url: string, sourceType: string): boolean => {
    const detected = detectSourceType(url);
    if (detected === sourceType) return true;
    if (detected === 'rss' && sourceType === 'web_crawl') return true;
    return false;
  };

  const handleAddSource = async () => {
    if (!newSource.name.trim() || !newSource.url.trim()) {
      showToast(t('radar.error.nameRequired'), 'error');
      return;
    }

    if (!validateSourceType(newSource.url, newSource.sourceType)) {
      const detected = detectSourceType(newSource.url);
      const typeNames: Record<string, string> = {
        rss: 'RSS',
        telegram: 'Telegram',
        web_crawl: 'Web Crawl'
      };
      showToast(t('radar.error.typeMismatch', { expected: typeNames[detected], actual: typeNames[newSource.sourceType] }), 'error');
      return;
    }

    setSaving(true);
    try {
      if (editingSource) {
        await updateRadarSource(editingSource.id, newSource);
        showToast(t('radar.sourceUpdated'), 'success');
      } else {
        await addRadarSource(newSource);
        showToast(t('radar.sourceAdded'), 'success');
      }
      setShowAddSource(false);
      setEditingSource(null);
      setNewSource({
        name: '',
        url: '',
        sourceType: 'rss',
        platform: '',
        checkInterval: 3600,
        proxyEnabled: false,
        proxyProtocol: 'http',
        proxyHost: '',
        proxyPort: 0,
        proxyUsername: '',
        proxyPassword: '',
      });
      loadSources();
    } catch {
      showToast(t('radar.error.addFailed'), 'error');
    } finally {
      setSaving(false);
    }
  };

  const handleCloseSourceModal = () => {
    setShowAddSource(false);
    setEditingSource(null);
    setNewSource({
      name: '',
      url: '',
      sourceType: 'rss',
      platform: '',
      checkInterval: 3600,
      proxyEnabled: false,
      proxyProtocol: 'http',
      proxyHost: '',
      proxyPort: 0,
      proxyUsername: '',
      proxyPassword: '',
    });
  };

  const onProxyProtocolChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    setNewSource(prev => ({ ...prev, proxyProtocol: e.target.value }));
  };

  const onProxyHostChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setNewSource(prev => ({ ...prev, proxyHost: e.target.value }));
  };

  const onProxyPortChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setNewSource(prev => ({ ...prev, proxyPort: parseInt(e.target.value) || 0 }));
  };

  const onProxyUsernameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setNewSource(prev => ({ ...prev, proxyUsername: e.target.value }));
  };

  const onProxyPasswordChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setNewSource(prev => ({ ...prev, proxyPassword: e.target.value }));
  };

  const tabs = [
    { key: 'all', label: t('radar.tabs.all'), count: items.length },
    { key: 'unread', label: t('radar.tabs.unread'), count: items.filter(i => i.status === 'unread').length },
    { key: 'collected', label: t('radar.tabs.collected'), count: items.filter(i => i.status === 'collected').length },
  ];

  return (
    <div className="flex flex-col h-full bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100">
      <header className="h-16 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between px-6 flex-shrink-0">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 bg-blue-600 rounded-xl flex items-center justify-center">
            <i className="fa-solid fa-satellite-dish text-white" />
          </div>
          <div>
            <h1 className="font-semibold text-lg leading-tight">{t('radar.title')}</h1>
            <p className="text-xs text-gray-500 dark:text-gray-400">{t('radar.subtitle')}</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <select
            value={timeRange}
            onChange={e => setTimeRange(e.target.value)}
            className="px-3 py-2 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 text-sm focus:outline-none focus:border-blue-500"
          >
            <option value="1d">{t('radar.timeRange.1d')}</option>
            <option value="7d">{t('radar.timeRange.7d')}</option>
            <option value="30d">{t('radar.timeRange.30d')}</option>
            <option value="all">{t('radar.timeRange.all')}</option>
          </select>
          <button
            onClick={() => setShowClearConfirm(true)}
            disabled={items.length === 0}
            className="px-3 py-2 text-sm border border-red-300 dark:border-red-700 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg flex items-center gap-2 transition disabled:opacity-50 disabled:cursor-not-allowed"
            title={t('radar.clearAll')}
          >
            <i className="fa-solid fa-trash-alt" />
            <span className="hidden sm:inline">{t('radar.clearAll')}</span>
          </button>
          <button
            onClick={handleScan}
            disabled={scanning}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white text-sm rounded-lg flex items-center gap-2 transition cursor-pointer disabled:cursor-not-allowed"
          >
            <i className={`fa-solid fa-rotate ${scanning ? 'animate-spin' : ''}`} />
            <span>{scanning ? t('radar.scanning') : t('radar.scanNow')}</span>
          </button>
        </div>
      </header>

      <div className="flex flex-1 min-h-0">
        <main className="flex-1 flex flex-col min-w-0">
          <div className="px-6 pt-4 pb-3 border-b border-gray-200 dark:border-gray-700 flex items-center gap-1 flex-shrink-0">
            {tabs.map(tab => (
              <button
                key={tab.key}
                onClick={() => { setActiveTab(tab.key); setPagination(prev => ({ ...prev, page: 1 })); }}
                className={`px-3 py-1.5 text-sm rounded-lg transition ${
                  activeTab === tab.key
                    ? 'bg-blue-600 text-white font-medium'
                    : 'hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-400'
                }`}
              >
                {tab.label}
                <span className={`ml-1 ${
                  activeTab === tab.key ? 'text-blue-200' : tab.key === 'unread' ? 'text-blue-500' : 'text-gray-400 dark:text-gray-500'
                }`}>
                  {tab.count}
                </span>
              </button>
            ))}
          </div>

          {/* 搜索工具栏 */}
          <div className="px-6 py-3 border-b border-gray-200 dark:border-gray-700 flex items-center gap-3 flex-shrink-0">
            <div className="flex-1">
              <input
                type="text"
                placeholder="搜索关键字..."
                value={searchKeyword}
                onChange={e => { setSearchKeyword(e.target.value); setPagination(prev => ({ ...prev, page: 1 })); }}
                onKeyDown={e => e.key === 'Enter' && loadItems()}
                className="w-full px-3 py-1.5 bg-gray-100 dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-sm focus:outline-none focus:border-blue-500"
              />
            </div>
            <select
              value={searchSourceIds}
              onChange={e => { setSearchSourceIds(e.target.value); setPagination(prev => ({ ...prev, page: 1 })); }}
              className="px-3 py-1.5 bg-gray-100 dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-sm focus:outline-none focus:border-blue-500"
            >
              <option value="">全部来源</option>
              {sources.map(s => (
                <option key={s.id} value={s.id}>{s.name}</option>
              ))}
            </select>
            <input
              type="date"
              value={searchStartDate}
              onChange={e => { setSearchStartDate(e.target.value); setPagination(prev => ({ ...prev, page: 1 })); }}
              className="px-3 py-1.5 bg-gray-100 dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-sm focus:outline-none focus:border-blue-500"
            />
            <span className="text-gray-400">-</span>
            <input
              type="date"
              value={searchEndDate}
              onChange={e => { setSearchEndDate(e.target.value); setPagination(prev => ({ ...prev, page: 1 })); }}
              className="px-3 py-1.5 bg-gray-100 dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-sm focus:outline-none focus:border-blue-500"
            />
            <button
              onClick={() => loadItems()}
              className="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white text-sm rounded-lg transition"
            >
              搜索
            </button>
            {(searchKeyword || searchSourceIds || searchStartDate || searchEndDate) && (
              <button
                onClick={() => { setSearchKeyword(''); setSearchSourceIds(''); setSearchStartDate(''); setSearchEndDate(''); setPagination(prev => ({ ...prev, page: 1 })); loadItems(); }}
                className="px-3 py-1.5 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 text-sm transition"
              >
                清除
              </button>
            )}
          </div>

          <div className="flex-1 overflow-auto p-6 space-y-3">
            {items.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full text-gray-400 dark:text-gray-500">
                <i className="fa-solid fa-inbox text-4xl mb-4" />
                <p>{t('radar.empty')}</p>
              </div>
            ) : (
              items.filter(item => activeTab === 'all' || item.status === activeTab).map(item => (
                <div
                  key={item.id}
                  className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl p-4 hover:border-gray-300 dark:hover:border-gray-600 transition"
                >
                  <div className="flex items-start gap-4">
                    <div className="w-10 h-10 bg-blue-100 dark:bg-blue-900/40 rounded-lg flex items-center justify-center text-blue-600 dark:text-blue-400 flex-shrink-0">
                      <i className="fa-solid fa-cube" />
                    </div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 flex-wrap">
                          <h3 className="font-semibold text-sm">{item.project_name || t('radar.unknownProject')}</h3>
                          {item.language && (
                            <span className="px-2 py-0.5 text-xs bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 rounded">{item.language}</span>
                          )}
                        </div>
                        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1.5 leading-relaxed">
                          {highlightLinks(item.description || t('radar.noDescription'))}
                        </p>
                        <p className="text-xs text-gray-400 dark:text-gray-500 mt-2">
                          <i className="fa-regular fa-clock mr-1" />
                          {item.published_at || item.fetched_at}
                          {item.source_name && (
                            <span className="ml-2 text-blue-500">来自 {item.source_name}</span>
                          )}
                        </p>
                      </div>
                      <div className="flex flex-col items-end gap-2 flex-shrink-0">
                        {(() => {
                          const supportedUrls = findSupportedUrls(item.description);
                          const hasImport = supportedUrls.length > 0;
                          return (
                            <>
                              {item.status === 'unread' && (
                                <button
                                  onClick={() => handleAction(item.id, 'collect')}
                                  className="px-3 py-1.5 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition cursor-pointer"
                                >
                                  <i className="fa-regular fa-bookmark text-xs mr-1" />
                                  {t('radar.collect')}
                                </button>
                              )}
                              {hasImport && (
                                <button
                                  onClick={() => handleImport(supportedUrls)}
                                  className="px-3 py-1.5 text-sm bg-green-600 hover:bg-green-700 text-white rounded-lg transition cursor-pointer"
                                >
                                  <i className="fa-solid fa-download text-xs mr-1" />
                                  导入
                                </button>
                              )}
                              {item.status === 'collected' && !hasImport && (
                                <span className="px-3 py-1.5 text-sm rounded-lg text-green-600 dark:text-green-400">
                                  <i className="fa-solid fa-bookmark mr-1" />{t('radar.collected')}
                                </span>
                              )}
                            </>
                          );
                        })()}
                      </div>
                  </div>
                </div>
              ))
            )}
          </div>

          {/* 分页控件 */}
          {pagination.totalPages > 1 && (
            <div className="px-6 py-3 border-t border-gray-200 dark:border-gray-700 flex items-center justify-between flex-shrink-0">
              <span className="text-sm text-gray-500 dark:text-gray-400">
                共 {pagination.total} 条，第 {pagination.page}/{pagination.totalPages} 页
              </span>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => setPagination(prev => ({ ...prev, page: prev.page - 1 }))}
                  disabled={pagination.page <= 1}
                  className="px-3 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed transition"
                >
                  上一页
                </button>
                <select
                  value={pagination.pageSize}
                  onChange={e => setPagination(prev => ({ ...prev, pageSize: Number(e.target.value), page: 1 }))}
                  className="px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800"
                >
                  <option value={10}>10/页</option>
                  <option value={20}>20/页</option>
                  <option value={50}>50/页</option>
                </select>
                <button
                  onClick={() => setPagination(prev => ({ ...prev, page: prev.page + 1 }))}
                  disabled={pagination.page >= pagination.totalPages}
                  className="px-3 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed transition"
                >
                  下一页
                </button>
              </div>
            </div>
          )}
        </main>

        <aside className="w-72 bg-gray-50 dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700 p-4 overflow-auto">
          <div className="flex items-center justify-between mb-3">
            <h2 className="font-medium text-sm">{t('radar.sources')}</h2>
            <button
              onClick={() => setShowAddSource(true)}
              className="text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 text-xs cursor-pointer"
            >
              <i className="fa-solid fa-plus mr-1" />
              {t('radar.addSource')}
            </button>
          </div>
          <div className="space-y-2">
            {sources.length === 0 ? (
              <p className="text-gray-400 dark:text-gray-500 text-sm text-center py-4">
                {t('radar.noSources')}
              </p>
            ) : (
              sources.map(source => (
                <div
                  key={source.id}
                  className="px-3 py-2.5 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2 flex-1 min-w-0">
                      <span className={`w-2 h-2 rounded-full flex-shrink-0 ${
                        source.last_status === 'success' ? 'bg-green-500' :
                        source.last_status === 'error' ? 'bg-red-500' : 'bg-gray-400 dark:bg-gray-500'
                      }`} />
                      <span className="text-sm font-medium truncate">{source.name}</span>
                    </div>
                    <div className="flex items-center gap-1 flex-shrink-0">
                      <button
                        onClick={() => handleEditSource(source)}
                        className="p-1 text-gray-400 hover:text-blue-500 dark:hover:text-blue-400"
                        title={t('radar.edit')}
                      >
                        <i className="fa-solid fa-pen-to-square text-xs" />
                      </button>
                      <button
                        onClick={() => handleDeleteSource(source.id)}
                        className="p-1 text-gray-400 hover:text-red-500 dark:hover:text-red-400"
                        title={t('radar.delete')}
                      >
                        <i className="fa-solid fa-trash text-xs" />
                      </button>
                    </div>
                  </div>
                  <p className="text-xs text-gray-400 dark:text-gray-500 mt-0.5">
                    {source.last_status === 'error' && source.last_error ? (
                      <span className="text-red-500" title={source.last_error}>Error: {source.last_error}</span>
                    ) : source.last_checked_at ? source.last_checked_at : t('radar.neverScanned')}
                  </p>
                </div>
              ))
            )}
          </div>
        </aside>
      </div>

      {showAddSource && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white dark:bg-gray-800 rounded-xl p-6 w-full max-w-lg max-h-[90vh] overflow-y-auto shadow-2xl border border-gray-200 dark:border-gray-700">
            <div className="flex items-center justify-between mb-4 flex-shrink-0">
              <h2 className="text-lg font-semibold dark:text-gray-100">
                {editingSource ? t('radar.editSource') : t('radar.addSource')}
              </h2>
              <button
                onClick={handleCloseSourceModal}
                className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
              >
                <i className="fa-solid fa-xmark" />
              </button>
            </div>

            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">{t('radar.sourceName')}</label>
                <input
                  type="text"
                  value={newSource.name}
                  onChange={e => setNewSource(prev => ({ ...prev, name: e.target.value }))}
                  placeholder={t('radar.sourceNamePlaceholder')}
                  className="w-full px-3 py-2 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:border-blue-500"
                />
              </div>

              <div>
                <label className="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">{t('radar.sourceType')}</label>
                <select
                  value={newSource.sourceType}
                  onChange={e => setNewSource(prev => ({ ...prev, sourceType: e.target.value }))}
                  className="w-full px-3 py-2 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500"
                >
                  <option value="rss">RSS Feed</option>
                  <option value="web_crawl">Web Crawl</option>
                  <option value="telegram">Telegram</option>
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium mb-1 text-gray-700 dark:text-gray-300">{t('radar.sourceUrl')}</label>
                <input
                  type="text"
                  value={newSource.url}
                  onChange={e => setNewSource(prev => ({ ...prev, url: e.target.value }))}
                  placeholder="https://..."
                  className="w-full px-3 py-2 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:outline-none focus:border-blue-500"
                />
              </div>

              <div className="border-t border-gray-200 dark:border-gray-700 pt-4">
                <label className="flex items-center gap-2 text-sm text-gray-700 dark:text-gray-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={newSource.proxyEnabled || false}
                    onChange={e => setNewSource(prev => ({ ...prev, proxyEnabled: e.target.checked }))}
                    className="rounded"
                  />
                  {t('radar.useProxy')}
                </label>

                {newSource.proxyEnabled && (
                  <div className="space-y-3 pl-6 mt-3">
                    <div className="grid grid-cols-3 gap-3">
                      <div>
                        <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">{t('radar.proxyProtocol')}</label>
                        <select
                          value={newSource.proxyProtocol || 'http'}
                          onChange={onProxyProtocolChange}
                          className="w-full px-2 py-1.5 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 text-sm focus:outline-none focus:border-blue-500"
                        >
                          <option value="http">HTTP</option>
                          <option value="https">HTTPS</option>
                          <option value="socks5">SOCKS5</option>
                        </select>
                      </div>
                      <div className="col-span-2">
                        <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">{t('radar.proxyHost')}</label>
                        <input
                          type="text"
                          value={newSource.proxyHost || ''}
                          onChange={onProxyHostChange}
                          placeholder="127.0.0.1"
                          className="w-full px-2 py-1.5 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 text-sm focus:outline-none focus:border-blue-500"
                        />
                      </div>
                    </div>
                    <div className="grid grid-cols-2 gap-3">
                      <div>
                        <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">{t('radar.proxyPort')}</label>
                        <input
                          type="number"
                          value={newSource.proxyPort || ''}
                          onChange={onProxyPortChange}
                          placeholder="7890"
                          className="w-full px-2 py-1.5 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 text-sm focus:outline-none focus:border-blue-500"
                        />
                      </div>
                      <div>
                        <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">{t('radar.proxyUsername')}</label>
                        <input
                          type="text"
                          value={newSource.proxyUsername || ''}
                          onChange={onProxyUsernameChange}
                          placeholder={t('radar.proxyUsernamePlaceholder')}
                          className="w-full px-2 py-1.5 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 text-sm focus:outline-none focus:border-blue-500"
                        />
                      </div>
                    </div>
                    <div>
                      <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">{t('radar.proxyPassword')}</label>
                      <input
                        type="password"
                        value={newSource.proxyPassword || ''}
                        onChange={onProxyPasswordChange}
                        placeholder={t('radar.proxyPasswordPlaceholder')}
                        className="w-full px-2 py-1.5 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 text-sm focus:outline-none focus:border-blue-500"
                      />
                    </div>
                  </div>
                )}
              </div>

              <div className="flex justify-end gap-2 pt-4 border-t border-gray-200 dark:border-gray-700 flex-shrink-0">
                <button
                  onClick={() => setShowAddSource(false)}
                  className="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 transition cursor-pointer"
                >
                  {t('common.cancel')}
                </button>
                <button
                  onClick={handleAddSource}
                  disabled={saving}
                  className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition cursor-pointer disabled:opacity-50"
                >
                  {saving ? t('common.saving') : t('common.save')}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {showClearConfirm && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white dark:bg-gray-800 rounded-xl p-6 max-w-md mx-4 shadow-2xl">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-12 h-12 bg-red-100 dark:bg-red-900/30 rounded-full flex items-center justify-center">
                <i className="fa-solid fa-exclamation-triangle text-2xl text-red-600 dark:text-red-400" />
              </div>
              <div>
                <h3 className="font-semibold text-lg text-gray-900 dark:text-gray-100">{t('radar.clearConfirmTitle')}</h3>
                <p className="text-sm text-gray-500 dark:text-gray-400">{t('radar.clearConfirmMessage')}</p>
              </div>
            </div>
            <div className="flex justify-end gap-3">
              <button
                onClick={() => setShowClearConfirm(false)}
                className="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition"
              >
                {t('common.cancel')}
              </button>
              <button
                onClick={handleClearAll}
                disabled={clearing}
                className="px-4 py-2 text-sm bg-red-600 hover:bg-red-700 text-white rounded-lg transition disabled:opacity-50"
              >
                {clearing ? t('common.clearing') : t('radar.clearConfirm')}
              </button>
            </div>
          </div>
        </div>
      )}

      {importUrl && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white dark:bg-gray-800 rounded-xl p-6 w-full max-w-lg mx-4 shadow-2xl">
            <h3 className="text-lg font-semibold mb-4">导入项目</h3>
            {importUrl.includes('\n') ? (
              <p className="text-sm text-gray-500 mb-4">检测到 {importUrl.split('\n').length} 个支持的链接</p>
            ) : (
              <p className="text-sm text-gray-500 mb-4">导入链接：{importUrl}</p>
            )}
            <div className="flex justify-end gap-3">
              <button
                onClick={() => setImportUrl(null)}
                className="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition"
              >
                {t('common.cancel')}
              </button>
              <button
                onClick={() => {
                  // TODO: 跳转到项目看板并传递URL
                  setImportUrl(null);
                  showToast('功能开发中：导入到项目看板', 'info');
                }}
                className="px-4 py-2 text-sm bg-green-600 hover:bg-green-700 text-white rounded-lg transition"
              >
                确认导入
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
