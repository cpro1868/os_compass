import { useState, useEffect, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { listRadarSources, addRadarSource, updateRadarSource, deleteRadarSource, getRadarItems, triggerRadarScan, radarItemAction, clearRadarAll, getSupportedPlatformDomains, RadarSource, RadarItem, RadarSourceInput, getRadarSchedule, updateRadarSchedule, RadarSchedule, RadarScheduleUpdate } from '../api/radar';
import { useToastStore } from '../stores/toastStore';
import { translate } from '../api';
import { markAsAd, AdMarkResult } from '../api/adPatterns';

// 获取一周前的日期（格式：YYYY-MM-DD）
const getOneWeekAgo = (): string => {
  const date = new Date();
  date.setDate(date.getDate() - 7);
  return date.toISOString().split('T')[0];
};

// 获取今天的日期（格式：YYYY-MM-DD）
const getToday = (): string => {
  return new Date().toISOString().split('T')[0];
};

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
  // 搜索相关状态（默认最近一周）
  const [searchKeyword, setSearchKeyword] = useState('');
  const [searchSourceIds, setSearchSourceIds] = useState<string>('');
  const [searchStartDate, setSearchStartDate] = useState<string>(getOneWeekAgo());
  const [searchEndDate, setSearchEndDate] = useState<string>(getToday());
  const [pagination, setPagination] = useState({ page: 1, pageSize: 20, total: 0, totalPages: 0 });
  const [deleteItemId, setDeleteItemId] = useState<number | null>(null);
  const [translatingId, setTranslatingId] = useState<number | null>(null);
  const [translatedContent, setTranslatedContent] = useState<Record<number, string>>({});
  const [adMarkItem, setAdMarkItem] = useState<RadarItem | null>(null);
  const [adMarkResult, setAdMarkResult] = useState<AdMarkResult | null>(null);
  const [adMarking, setAdMarking] = useState(false);

  const [schedule, setSchedule] = useState<RadarSchedule | null>(null);
  const [showScheduleSettings, setShowScheduleSettings] = useState(false);
  const [scheduleUpdate, setScheduleUpdate] = useState<RadarScheduleUpdate>({});

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
      // 加载全部数据，用于显示各 tab 的数量
      const result = await getRadarItems(undefined);
      if (result === undefined || result === null) {
        console.error('[RadarInbox] getRadarItems returned null/undefined');
        showToast('加载数据失败：返回数据为空', 'error');
        return;
      }
      console.log('[RadarInbox] getRadarItems returned:', result.items.length, 'items, total:', result.total);
      setItems(result.items);
      // 使用后端返回的分页信息
      setPagination(prev => ({ 
        ...prev, 
        page: 1, 
        total: result.total, 
        totalPages: result.total_pages 
      }));
    } catch (e) {
      console.error('[RadarInbox] Failed to load items:', e);
      showToast('加载情报失败: ' + String(e), 'error');
    }
  }, []);

  // 前端展示筛选（根据状态和搜索条件过滤已加载的数据）
  const filteredItems = useMemo(() => {
    return items.filter(item => {
      // 状态筛选
      if (activeTab !== 'all' && item.status !== activeTab) return false;
      // 关键字筛选
      if (searchKeyword) {
        const kw = searchKeyword.toLowerCase();
        const matchName = item.project_name?.toLowerCase().includes(kw);
        const matchDesc = item.description?.toLowerCase().includes(kw);
        if (!matchName && !matchDesc) return false;
      }
      // 来源筛选
      if (searchSourceIds && String(item.source_id) !== searchSourceIds) {
        return false;
      }
      // 日期筛选
      const itemDate = item.published_at || item.fetched_at;
      if (itemDate) {
        if (searchStartDate && itemDate < searchStartDate) return false;
        if (searchEndDate) {
          const endDate = new Date(searchEndDate);
          endDate.setDate(endDate.getDate() + 1); // 包含结束日期当天
          if (itemDate > endDate.toISOString()) return false;
        }
      }
      return true;
    });
  }, [items, activeTab, searchKeyword, searchSourceIds, searchStartDate, searchEndDate]);

  // 前端分页
  const paginatedItems = useMemo(() => {
    const filteredCount = filteredItems.length;
    const totalPages = Math.max(1, Math.ceil(filteredCount / pagination.pageSize));
    const validPage = Math.min(Math.max(1, pagination.page), totalPages);
    const start = (validPage - 1) * pagination.pageSize;
    const end = start + pagination.pageSize;
    return filteredItems.slice(start, end);
  }, [filteredItems, pagination.page, pagination.pageSize]);

  // 筛选变化时重置页码
  useEffect(() => {
    setPagination(prev => ({ ...prev, page: 1 }));
  }, [activeTab, searchKeyword, searchSourceIds, searchStartDate, searchEndDate]);

  // 移除错误的自动加载 useEffect

  useEffect(() => {
    loadSources();
    loadItems();
    loadSupportedDomains();
    loadSchedule();

    const handleVaultChanged = () => {
      console.log('[RadarInbox] vault changed, reloading...');
      loadSources();
      loadItems();
      loadSupportedDomains();
      loadSchedule();
    };
    window.addEventListener('vault-changed', handleVaultChanged);
    return () => window.removeEventListener('vault-changed', handleVaultChanged);
  }, [loadSources, loadItems, loadSupportedDomains]);

  const loadSchedule = async () => {
    try {
      const data = await getRadarSchedule();
      setSchedule(data);
    } catch (e) {
      console.error('Failed to load schedule:', e);
    }
  };

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

  const handleDeleteItem = (itemId: number) => {
    setDeleteItemId(itemId);
  };

  const confirmDeleteItem = async () => {
    if (deleteItemId === null) return;
    try {
      await radarItemAction(deleteItemId, 'delete');
      showToast('删除成功', 'success');
      setDeleteItemId(null);
      loadItems();
    } catch {
      showToast('删除失败', 'error');
      setDeleteItemId(null);
    }
  };

  const handleTranslate = async (item: RadarItem) => {
    const textToTranslate = item.description || item.project_name || '';
    if (!textToTranslate.trim()) {
      showToast('没有可翻译的内容', 'info');
      return;
    }

    setTranslatingId(item.id);
    try {
      const currentLang = localStorage.getItem('i18nextLng') || 'zh';
      const targetLang = currentLang.startsWith('zh') ? 'zh-CN' : 'en';
      const translated = await translate(textToTranslate, targetLang);
      setTranslatedContent(prev => ({ ...prev, [item.id]: translated }));
      showToast('翻译成功', 'success');
    } catch (e) {
      console.error('Translation failed:', e);
      showToast('翻译失败', 'error');
    } finally {
      setTranslatingId(null);
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
          <div className="flex items-center gap-1 px-3 py-2 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg">
            <i className="fa-regular fa-clock text-gray-500 dark:text-gray-400 text-sm" />
            <span className={`text-sm ${schedule?.enabled ? 'text-blue-600 dark:text-blue-400 font-medium' : 'text-gray-500 dark:text-gray-400'}`}>
              {schedule?.enabled ? (schedule.mode === 'interval' ? `每${schedule.interval_seconds / 60}分钟` : `每${schedule.custom_value}${schedule.custom_unit === 'second' ? '秒' : schedule.custom_unit === 'minute' ? '分钟' : '小时'}`) : t('radar.autoCollect')}
            </span>
            <label className="relative inline-flex items-center cursor-pointer ml-1">
              <input
                type="checkbox"
                checked={schedule?.enabled || false}
                onChange={async (e) => {
                  try {
                    await updateRadarSchedule({ enabled: e.target.checked });
                    await loadSchedule();
                    showToast(e.target.checked ? t('radar.autoCollectEnabled') : t('radar.autoCollectDisabled'), 'success');
                  } catch (err) {
                    showToast(t('radar.autoCollectError'), 'error');
                  }
                }}
                className="sr-only peer"
              />
              <div className="w-9 h-5 bg-gray-300 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-blue-600 dark:peer-checked:bg-blue-500" />
            </label>
            <button
              onClick={() => {
                setScheduleUpdate({
                  enabled: schedule?.enabled,
                  mode: schedule?.mode as 'interval' | 'custom',
                  interval_seconds: schedule?.interval_seconds,
                  custom_unit: schedule?.custom_unit as 'second' | 'minute' | 'hour' | undefined,
                  custom_value: schedule?.custom_value ?? undefined,
                  notification_enabled: schedule?.notification_enabled,
                  system_notification: schedule?.system_notification,
                  badge_notification: schedule?.badge_notification,
                });
                setShowScheduleSettings(true);
              }}
              className="ml-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition"
              title={t('radar.scheduleSettings')}
            >
              <i className="fa-solid fa-gear text-sm" />
            </button>
          </div>
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
            {(searchKeyword || searchSourceIds || searchStartDate !== getOneWeekAgo() || searchEndDate !== getToday()) && (
              <button
                onClick={() => { setSearchKeyword(''); setSearchSourceIds(''); setSearchStartDate(getOneWeekAgo()); setSearchEndDate(getToday()); }}
                className="px-3 py-1.5 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 text-sm transition"
              >
                重置
              </button>
            )}
          </div>

          <div className="flex-1 overflow-auto p-6 space-y-3">
            {paginatedItems.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full text-gray-400 dark:text-gray-500">
                <i className="fa-solid fa-inbox text-4xl mb-4" />
                <p>{searchKeyword || searchSourceIds || searchStartDate ? '无匹配结果' : t('radar.empty')}</p>
              </div>
            ) : (
              paginatedItems.map(item => (
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
                        {translatedContent[item.id] ? (
                          <div className="mt-1.5">
                            <div className="flex items-center gap-2 mb-1">
                              <span className="px-2 py-0.5 text-xs bg-purple-100 dark:bg-purple-900/40 text-purple-600 dark:text-purple-300 rounded">
                                <i className="fa-solid fa-language mr-1" />译文
                              </span>
                              <button
                                onClick={() => {
                                  setTranslatedContent(prev => {
                                    const next = { ...prev };
                                    delete next[item.id];
                                    return next;
                                  });
                                }}
                                className="text-xs text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                              >
                                显示原文
                              </button>
                            </div>
                            <p className="text-sm text-gray-500 dark:text-gray-400 leading-relaxed">
                              {highlightLinks(translatedContent[item.id])}
                            </p>
                          </div>
                        ) : (
                          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1.5 leading-relaxed">
                            {highlightLinks(item.description || t('radar.noDescription'))}
                          </p>
                        )}
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
                              <div className="flex items-center gap-1">
                                <button
                                  onClick={() => handleTranslate(item)}
                                  disabled={translatingId === item.id}
                                  className="p-1.5 text-gray-400 hover:text-purple-500 dark:hover:text-purple-400 hover:bg-purple-50 dark:hover:bg-purple-900/20 rounded-lg transition disabled:opacity-50"
                                  title="翻译"
                                >
                                  {translatingId === item.id ? (
                                    <i className="fa-solid fa-spinner fa-spin text-xs" />
                                  ) : (
                                    <i className="fa-solid fa-language text-xs" />
                                  )}
                                </button>
                                <button
                                  onClick={() => {
                                    const text = `${item.project_name || ''}\n${translatedContent[item.id] || item.description || ''}\n${item.project_url || ''}`;
                                    navigator.clipboard.writeText(text);
                                    showToast('已复制到剪贴板', 'success');
                                  }}
                                  className="p-1.5 text-gray-400 hover:text-blue-500 dark:hover:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg transition"
                                  title="复制"
                                >
                                  <i className="fa-regular fa-copy text-xs" />
                                </button>
                                <button
                                  onClick={() => handleDeleteItem(item.id)}
                                  className="p-1.5 text-gray-400 hover:text-red-500 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition"
                                  title="删除"
                                >
                                  <i className="fa-solid fa-trash text-xs" />
                                </button>
                              </div>
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
                              <button
                                onClick={() => {
                                  setAdMarkItem(item);
                                  setAdMarkResult(null);
                                }}
                                className="px-3 py-1.5 text-sm bg-red-600 hover:bg-red-700 text-white rounded-lg transition cursor-pointer"
                              >
                                <i className="fa-solid fa-ban text-xs mr-1" />
                                标记广告
                              </button>
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
                共 {filteredItems.length} 条，第 {pagination.page}/{Math.ceil(filteredItems.length / pagination.pageSize) || 1} 页
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
                  disabled={pagination.page >= Math.ceil(filteredItems.length / pagination.pageSize)}
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

      {deleteItemId !== null && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white dark:bg-gray-800 rounded-xl p-6 max-w-md mx-4 shadow-2xl">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-12 h-12 bg-red-100 dark:bg-red-900/30 rounded-full flex items-center justify-center">
                <i className="fa-solid fa-trash text-2xl text-red-600 dark:text-red-400" />
              </div>
              <div>
                <h3 className="font-semibold text-lg text-gray-900 dark:text-gray-100">确认删除</h3>
                <p className="text-sm text-gray-500 dark:text-gray-400">确定要删除这条雷达记录吗？此操作不可撤销。</p>
              </div>
            </div>
            <div className="flex justify-end gap-3">
              <button
                onClick={() => setDeleteItemId(null)}
                className="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition"
              >
                取消
              </button>
              <button
                onClick={confirmDeleteItem}
                className="px-4 py-2 text-sm bg-red-600 hover:bg-red-700 text-white rounded-lg transition"
              >
                删除
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

      {adMarkItem && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-white dark:bg-gray-800 rounded-xl p-6 w-full max-w-lg mx-4 shadow-2xl">
            <h3 className="text-lg font-semibold mb-4 flex items-center">
              <i className="fa-solid fa-ban text-red-500 mr-2" />
              标记广告
            </h3>

            {!adMarkResult ? (
              <>
                <div className="mb-4 p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                  <p className="text-sm font-medium mb-2">即将分析以下内容：</p>
                  <p className="text-sm text-gray-600 dark:text-gray-300 truncate">{adMarkItem.project_name}</p>
                  <p className="text-xs text-gray-500 mt-1 line-clamp-2">{adMarkItem.description}</p>
                </div>
                <p className="text-sm text-gray-500 mb-4">
                  点击确认后，系统将使用 LLM 分析内容是否为广告，并学习屏蔽规则。
                </p>
                <div className="flex justify-end gap-3">
                  <button
                    onClick={() => setAdMarkItem(null)}
                    className="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition"
                  >
                    取消
                  </button>
                  <button
                    onClick={async () => {
                      if (!adMarkItem) return;
                      setAdMarking(true);
                      try {
                        const result = await markAsAd(
                          String(adMarkItem.id),
                          adMarkItem.project_name || '',
                          adMarkItem.description || '',
                          adMarkItem.project_url || ''
                        );
                        setAdMarkResult(result);
                        if (result.success) {
                          showToast(result.message, 'success');
                        }
                      } catch (e) {
                        showToast('标记失败: ' + String(e), 'error');
                      } finally {
                        setAdMarking(false);
                      }
                    }}
                    disabled={adMarking}
                    className="px-4 py-2 text-sm bg-red-600 hover:bg-red-700 text-white rounded-lg transition disabled:opacity-50"
                  >
                    {adMarking ? '分析中...' : '确认分析'}
                  </button>
                </div>
              </>
            ) : (
              <>
                <div className="mb-4">
                  <div className={`p-4 rounded-lg ${adMarkResult.llm_judgment?.is_ad ? 'bg-red-50 dark:bg-red-900/20' : 'bg-green-50 dark:bg-green-900/20'}`}>
                    <div className="flex items-center mb-2">
                      <span className={`px-2 py-1 text-xs font-medium rounded ${
                        adMarkResult.llm_judgment?.is_ad
                          ? 'bg-red-100 text-red-700 dark:bg-red-900/50 dark:text-red-300'
                          : 'bg-green-100 text-green-700 dark:bg-green-900/50 dark:text-green-300'
                      }`}>
                        {adMarkResult.llm_judgment?.is_ad ? '判定为广告' : '判定为正常内容'}
                      </span>
                      <span className="ml-2 text-sm text-gray-500">
                        置信度: {Math.round((adMarkResult.llm_judgment?.confidence || 0) * 100)}%
                      </span>
                    </div>
                    {adMarkResult.llm_judgment?.reasoning && (
                      <p className="text-sm text-gray-600 dark:text-gray-300 mt-2">
                        {adMarkResult.llm_judgment.reasoning}
                      </p>
                    )}
                    {adMarkResult.llm_judgment?.ad_keywords && adMarkResult.llm_judgment.ad_keywords.length > 0 && (
                      <div className="mt-3">
                        <p className="text-xs text-gray-500 mb-1">提取的关键词：</p>
                        <div className="flex flex-wrap gap-1">
                          {adMarkResult.llm_judgment.ad_keywords.map((kw: string, i: number) => (
                            <span key={i} className="px-2 py-0.5 text-xs bg-red-100 text-red-700 dark:bg-red-900/50 dark:text-red-300 rounded">
                              {kw}
                            </span>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                </div>
                <div className="flex justify-end gap-3">
                  <button
                    onClick={() => {
                      setAdMarkItem(null);
                      setAdMarkResult(null);
                    }}
                    className="px-4 py-2 text-sm bg-gray-600 hover:bg-gray-700 text-white rounded-lg transition"
                  >
                    关闭
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      )}

      {showScheduleSettings && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={() => setShowScheduleSettings(false)}>
          <div className="bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-md mx-4" onClick={e => e.stopPropagation()}>
            <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
              <h2 className="text-lg font-semibold flex items-center gap-2">
                <i className="fa-regular fa-clock text-blue-500" />
                {t('radar.scheduleSettings')}
              </h2>
              <button onClick={() => setShowScheduleSettings(false)} className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200">
                <i className="fa-solid fa-xmark text-xl" />
              </button>
            </div>

            <div className="p-4 space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{t('radar.collectMode')}</label>
                <div className="space-y-2">
                  <label className="flex items-center gap-3 p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition">
                    <input
                      type="radio"
                      name="collectMode"
                      value="interval"
                      checked={scheduleUpdate.mode === 'interval'}
                      onChange={() => setScheduleUpdate(prev => ({ ...prev, mode: 'interval' }))}
                      className="w-4 h-4 text-blue-600 bg-gray-300 border-gray-400 focus:ring-blue-500 dark:bg-gray-700"
                    />
                    <div className="flex-1">
                      <span className="text-sm font-medium">{t('radar.intervalMode')}</span>
                      <p className="text-xs text-gray-500">{t('radar.intervalModeHint')}</p>
                    </div>
                  </label>
                  <div className={`pl-7 ${scheduleUpdate.mode === 'interval' ? '' : 'hidden'}`}>
                    <select
                      value={scheduleUpdate.interval_seconds}
                      onChange={(e) => setScheduleUpdate(prev => ({ ...prev, interval_seconds: Number(e.target.value) }))}
                      className="w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-sm text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500"
                    >
                      <option value={300}>{t('radar.interval.5min')}</option>
                      <option value={900}>{t('radar.interval.15min')}</option>
                      <option value={1800}>{t('radar.interval.30min')}</option>
                      <option value={3600}>{t('radar.interval.1h')}</option>
                      <option value={7200}>{t('radar.interval.2h')}</option>
                      <option value={21600}>{t('radar.interval.6h')}</option>
                      <option value={43200}>{t('radar.interval.12h')}</option>
                      <option value={86400}>{t('radar.interval.24h')}</option>
                    </select>
                  </div>

                  <label className="flex items-center gap-3 p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition">
                    <input
                      type="radio"
                      name="collectMode"
                      value="custom"
                      checked={scheduleUpdate.mode === 'custom'}
                      onChange={() => setScheduleUpdate(prev => ({ ...prev, mode: 'custom' }))}
                      className="w-4 h-4 text-blue-600 bg-gray-300 border-gray-400 focus:ring-blue-500 dark:bg-gray-700"
                    />
                    <div className="flex-1">
                      <span className="text-sm font-medium">{t('radar.customMode')}</span>
                      <p className="text-xs text-gray-500">{t('radar.customModeHint')}</p>
                    </div>
                  </label>
                  <div className={`pl-7 ${scheduleUpdate.mode === 'custom' ? '' : 'hidden'}`}>
                    <div className="flex items-center gap-2">
                      <select
                        value={scheduleUpdate.custom_unit || 'minute'}
                        onChange={(e) => setScheduleUpdate(prev => ({ ...prev, custom_unit: e.target.value as 'second' | 'minute' | 'hour' }))}
                        className="px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-sm text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500"
                      >
                        <option value="second">{t('radar.unit.second')}</option>
                        <option value="minute">{t('radar.unit.minute')}</option>
                        <option value="hour">{t('radar.unit.hour')}</option>
                      </select>
                      <span className="text-sm text-gray-500">{t('radar.every')}</span>
                      <input
                        type="number"
                        value={scheduleUpdate.custom_value || 1}
                        onChange={(e) => setScheduleUpdate(prev => ({ ...prev, custom_value: Number(e.target.value) }))}
                        min={1}
                        max={scheduleUpdate.custom_unit === 'hour' ? 23 : 59}
                        className="w-16 px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-sm text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500 text-center"
                      />
                      <span className="text-sm text-gray-500">
                        {scheduleUpdate.custom_unit === 'second' ? t('radar.unit.second') : scheduleUpdate.custom_unit === 'minute' ? t('radar.unit.minute') : t('radar.unit.hour')}
                      </span>
                    </div>
                  </div>
                </div>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{t('radar.notificationSettings')}</label>
                <div className="space-y-2">
                  <label className="flex items-center gap-3 p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition">
                    <input
                      type="checkbox"
                      checked={scheduleUpdate.notification_enabled ?? true}
                      onChange={(e) => setScheduleUpdate(prev => ({ ...prev, notification_enabled: e.target.checked }))}
                      className="w-4 h-4 text-blue-600 bg-gray-300 border-gray-400 rounded focus:ring-blue-500 dark:bg-gray-700"
                    />
                    <div className="flex items-center gap-2">
                      <i className="fa-solid fa-bell text-gray-400" />
                      <span className="text-sm">{t('radar.enableNotification')}</span>
                    </div>
                  </label>
                  <div className={`pl-7 space-y-2 ${scheduleUpdate.notification_enabled ? '' : 'hidden'}`}>
                    <label className="flex items-center gap-3 p-2 bg-gray-100/50 dark:bg-gray-800/50 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700/50 transition">
                      <input
                        type="checkbox"
                        checked={scheduleUpdate.system_notification ?? true}
                        onChange={(e) => setScheduleUpdate(prev => ({ ...prev, system_notification: e.target.checked }))}
                        className="w-4 h-4 text-blue-600 bg-gray-300 border-gray-400 rounded focus:ring-blue-500 dark:bg-gray-700"
                      />
                      <span className="text-sm text-gray-600 dark:text-gray-300">{t('radar.systemNotification')}</span>
                    </label>
                    <label className="flex items-center gap-3 p-2 bg-gray-100/50 dark:bg-gray-800/50 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700/50 transition">
                      <input
                        type="checkbox"
                        checked={scheduleUpdate.badge_notification ?? true}
                        onChange={(e) => setScheduleUpdate(prev => ({ ...prev, badge_notification: e.target.checked }))}
                        className="w-4 h-4 text-blue-600 bg-gray-300 border-gray-400 rounded focus:ring-blue-500 dark:bg-gray-700"
                      />
                      <span className="text-sm text-gray-600 dark:text-gray-300">{t('radar.badgeNotification')}</span>
                    </label>
                  </div>
                </div>
              </div>

              {schedule && (
                <div className="p-3 bg-gray-50 dark:bg-gray-900 rounded-lg">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <span className={`w-2 h-2 rounded-full ${schedule.enabled ? 'bg-green-500' : 'bg-gray-400'}`} />
                      <span className="text-sm text-gray-600 dark:text-gray-400">
                        {schedule.enabled ? t('radar.status.running') : t('radar.status.stopped')}
                      </span>
                    </div>
                    {schedule.next_run_at && (
                      <span className="text-xs text-gray-500">
                        {t('radar.nextRun')}: {schedule.next_run_at}
                      </span>
                    )}
                  </div>
                </div>
              )}
            </div>

            <div className="flex items-center justify-end gap-2 p-4 border-t border-gray-200 dark:border-gray-700">
              <button
                onClick={() => setShowScheduleSettings(false)}
                className="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 border border-gray-300 dark:border-gray-600 rounded-lg transition"
              >
                {t('common.cancel')}
              </button>
              <button
                onClick={async () => {
                  try {
                    await updateRadarSchedule(scheduleUpdate);
                    await loadSchedule();
                    setShowScheduleSettings(false);
                    showToast(t('radar.scheduleSaved'), 'success');
                  } catch (e) {
                    showToast(t('radar.scheduleSaveError') + ': ' + String(e), 'error');
                  }
                }}
                className="px-4 py-2 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition"
              >
                {t('common.save')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
