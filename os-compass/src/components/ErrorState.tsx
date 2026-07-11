interface ErrorStateProps {
  title?: string;
  message: string;
  onRetry?: () => void;
}

export function ErrorState({ title = "加载失败", message, onRetry }: ErrorStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-16 text-center">
      <div className="w-20 h-20 bg-red-50 dark:bg-red-950 rounded-2xl flex items-center justify-center mb-4">
        <i className="fa-solid fa-circle-exclamation text-3xl text-red-400 dark:text-red-500"></i>
      </div>
      <h3 className="text-lg font-medium text-gray-700 dark:text-gray-300 mb-1">{title}</h3>
      <p className="text-sm text-gray-500 dark:text-gray-400 mb-4 max-w-sm break-all">{message}</p>
      {onRetry && (
        <button
          onClick={onRetry}
          className="px-4 py-2 border dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 text-sm flex items-center gap-2"
        >
          <i className="fa-solid fa-arrows-rotate"></i>重试
        </button>
      )}
    </div>
  );
}
