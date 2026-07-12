import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { listRadarSources, getRadarItems, triggerRadarScan, radarItemAction, RadarSource, RadarItem } from '../api/radar';
import { useToast } from '../hooks/useToast';

export function RadarInbox() {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const [sources, setSources] = useState<RadarSource[]>([]);
  const [items, setItems] = useState<RadarItem[]>([]);
  const [activeTab, setActiveTab] = useState<string>('all');
  const [scanning, setScanning] = useState(false);

  const loadSources = useCallback(async () => {
    try {
      const data = await listRadarSources();
      setSources(data);
    } catch {
      showToast(t('radar.error.loadFailed'), 'error');
    }
  }, [t, showToast]);

  const loadItems = useCallback(async () => {
    try {
      const status = activeTab === 'all' ? undefined : activeTab;
      const data = await getRadarItems(status);
      setItems(data);
    } catch {
      showToast(t('radar.error.loadFailed'), 'error');
    }
  }, [activeTab, t, showToast]);

  useEffect(() => {
    loadSources();
    loadItems();
  }, [loadSources, loadItems]);

  const handleScan = async () => {
    setScanning(true);
    try {
      const result = await triggerRadarScan();
      showToast(t('radar.scanComplete', { count: result.newItems }), 'success');
      loadItems();
    } catch {
      showToast(t('radar.error.scanFailed'), 'error');
    } finally {
      setScanning(false);
    }
  };

  const handleAction = async (itemId: number, action: 'import' | 'ignore' | 'blacklist') => {
    try {
      await radarItemAction(itemId, action);
      showToast(t(`radar.action.${action}Success`), 'success');
      loadItems();
    } catch {
      showToast(t('radar.error.actionFailed'), 'error');
    }
  };

  const tabs = [
    { key: 'all', label: t('radar.tabs.all'), count: items.length },
    { key: 'unread', label: t('radar.tabs.unread'), count: items.filter(i => i.status === 'unread').length },
    { key: 'imported', label: t('radar.tabs.imported'), count: items.filter(i => i.status === 'imported').length },
    { key: 'ignored', label: t('radar.tabs.ignored'), count: items.filter(i => i.status === 'ignored').length },
  ];

  return (
    <div className="flex flex-col h-full bg-gray-900 text-gray-100">
      <header className="h-16 bg-gray-800 border-b border-gray-700 flex items-center justify-between px-6 flex-shrink-0">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 bg-blue-600 rounded-xl flex items-center justify-center">
            <i className="fa-solid fa-satellite-dish text-white" />
          </div>
          <div>
            <h1 className="font-semibold text-lg leading-tight">{t('radar.title')}</h1>
            <p className="text-xs text-gray-400">{t('radar.subtitle')}</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleScan}
            disabled={scanning}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 text-white text-sm rounded-lg flex items-center gap-2 transition cursor-pointer disabled:cursor-not-allowed"
          >
            <i className={`fa-solid fa-rotate ${scanning ? 'spinning' : ''}`} />
            <span>{scanning ? t('radar.scanning') : t('radar.scanNow')}</span>
          </button>
        </div>
      </header>

      <div className="flex flex-1 min-h-0">
        <main className="flex-1 flex flex-col min-w-0">
          <div className="px-6 pt-4 pb-3 border-b border-gray-700 flex items-center gap-1 flex-shrink-0">
            {tabs.map(tab => (
              <button
                key={tab.key}
                onClick={() => setActiveTab(tab.key)}
                className={`px-3 py-1.5 text-sm rounded-lg transition ${
                  activeTab === tab.key
                    ? 'bg-gray-700 text-white font-medium'
                    : 'hover:bg-gray-700/60 text-gray-400 hover:text-white'
                }`}
              >
                {tab.label}
                <span className={activeTab === tab.key ? 'text-gray-400 ml-1' : `ml-1 ${
                  tab.key === 'unread' ? 'text-blue-400' : 'text-gray-500'
                }`}>
                  {tab.count}
                </span>
              </button>
            ))}
          </div>

          <div className="flex-1 overflow-auto p-6 space-y-3">
            {items.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full text-gray-500">
                <i className="fa-solid fa-inbox text-4xl mb-4" />
                <p>{t('radar.empty')}</p>
              </div>
            ) : (
              items.map(item => (
                <div
                  key={item.id}
                  className="bg-gray-800 border border-gray-700 rounded-xl p-4 hover:border-gray-600 transition"
                >
                  <div className="flex items-start gap-4">
                    <div className="w-10 h-10 bg-blue-900/40 rounded-lg flex items-center justify-center text-blue-400 flex-shrink-0">
                      <i className="fa-solid fa-cube" />
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 flex-wrap">
                        <h3 className="font-semibold text-sm">{item.project_name || t('radar.unknownProject')}</h3>
                        {item.language && (
                          <span className="px-2 py-0.5 text-xs bg-gray-700 text-gray-300 rounded">{item.language}</span>
                        )}
                      </div>
                      <p className="text-sm text-gray-400 mt-1.5 leading-relaxed">
                        {item.description || t('radar.noDescription')}
                      </p>
                      <p className="text-xs text-gray-500 mt-2">
                        <i className="fa-regular fa-clock mr-1" />
                        {item.fetched_at}
                      </p>
                    </div>
                    <div className="flex flex-col items-end gap-2 flex-shrink-0">
                      {item.status === 'unread' && (
                        <>
                          <button
                            onClick={() => handleAction(item.id, 'import')}
                            className="px-3 py-1.5 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition cursor-pointer"
                          >
                            <i className="fa-solid fa-download text-xs mr-1" />
                            {t('radar.import')}
                          </button>
                          <button
                            onClick={() => handleAction(item.id, 'ignore')}
                            className="px-3 py-1.5 text-sm text-gray-400 hover:text-gray-200 transition cursor-pointer"
                          >
                            <i className="fa-regular fa-thumbs-down text-xs mr-1" />
                            {t('radar.ignore')}
                          </button>
                        </>
                      )}
                      {item.status !== 'unread' && (
                        <span className={`px-3 py-1.5 text-sm rounded-lg ${
                          item.status === 'imported' ? 'text-green-400' : 'text-gray-500'
                        }`}>
                          {item.status === 'imported' ? (
                            <>
                              <i className="fa-solid fa-check mr-1" />
                              {t('radar.imported')}
                            </>
                          ) : (
                            <>
                              <i className="fa-solid fa-ban mr-1" />
                              {t('radar.ignored')}
                            </>
                          )}
                        </span>
                      )}
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        </main>

        <aside className="w-72 bg-gray-800 border-l border-gray-700 p-4 overflow-auto">
          <div className="flex items-center justify-between mb-3">
            <h2 className="font-medium text-sm">{t('radar.sources')}</h2>
            <button className="text-blue-400 hover:text-blue-300 text-xs">
              <i className="fa-solid fa-plus mr-1" />
              {t('radar.addSource')}
            </button>
          </div>
          <div className="space-y-2">
            {sources.length === 0 ? (
              <p className="text-gray-500 text-sm text-center py-4">
                {t('radar.noSources')}
              </p>
            ) : (
              sources.map(source => (
                <div
                  key={source.id}
                  className="px-3 py-2.5 rounded-lg hover:bg-gray-700/60 transition cursor-pointer"
                >
                  <div className="flex items-center gap-2">
                    <span className={`w-2 h-2 rounded-full ${
                      source.last_status === 'success' ? 'bg-green-500' :
                      source.last_status === 'error' ? 'bg-red-500' : 'bg-gray-500'
                    }`} />
                    <span className="text-sm font-medium truncate">{source.name}</span>
                  </div>
                  <p className="text-xs text-gray-500 mt-0.5">
                    {source.last_checked_at ? source.last_checked_at : t('radar.neverScanned')}
                  </p>
                </div>
              ))
            )}
          </div>
        </aside>
      </div>
    </div>
  );
}
