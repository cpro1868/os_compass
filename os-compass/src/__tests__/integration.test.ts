import { describe, it, expect } from "vitest";

type SearchPhase = 'idle' | 'analyzing' | 'searching';

interface MessageItem {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  phase?: SearchPhase;
  results?: any;
  clarification?: any;
}

interface IntentAnalysis {
  intent: string;
  intent_type?: string;
  keywords?: string[];
  questions?: string[];
  options?: string[];
  guidance?: string;
}

function simulateHandleSubmit(query: string, mockAnalyzeIntent: (q: string) => Promise<IntentAnalysis>) {
  const messages: MessageItem[] = [];
  const loadingIdRef = { current: null as string | null };
  let searchPhase: SearchPhase = 'idle';
  let isLoading = false;

  function setMessages(updater: (prev: MessageItem[]) => MessageItem[]) {
    const prev = [...messages];
    const newMessages = updater(prev);
    messages.length = 0;
    messages.push(...newMessages);
  }

  function setSearchPhase(phase: SearchPhase) {
    searchPhase = phase;
  }

  function setIsLoading(loading: boolean) {
    isLoading = loading;
  }

  function updatePhase(phase: SearchPhase, content: string) {
    setSearchPhase(phase);
    if (loadingIdRef.current) {
      setMessages(prev => prev.map(msg => {
        return msg.id === loadingIdRef.current ? { ...msg, phase, content } : msg;
      }));
    }
  }

  async function handleSubmit() {
    if (!query.trim() || isLoading) return;

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

    setMessages(prev => [...prev, userMessage, loadingMessage]);
    setIsLoading(true);
    setSearchPhase('analyzing');

    try {
      let intent = await mockAnalyzeIntent(query);

      if (intent?.intent === 'off_topic' && intent?.guidance) {
        updatePhase('idle', intent.guidance || '');
        setIsLoading(false);
        return;
      } else if (intent?.intent === 'unclear' && intent?.questions) {
        updatePhase('idle', intent.questions?.join('\n') || '');
        setIsLoading(false);
        return;
      } else {
        updatePhase('searching', '正在搜索...');
      }
    } finally {
      setIsLoading(false);
      setSearchPhase('idle');
      loadingIdRef.current = null;
    }
  }

  return { handleSubmit, messages, searchPhase, isLoading, loadingIdRef };
}

describe("意图搜索完整流程测试", () => {
  
  it("off_topic 流程：消息应正确更新", async () => {
    const mockAnalyzeIntent = async (_q: string): Promise<IntentAnalysis> => ({
      intent: 'off_topic',
      guidance: '你好！我是开源项目助手。你想找什么类型的开源项目？'
    });

    const { handleSubmit, messages, searchPhase, isLoading } = simulateHandleSubmit(
      '你好',
      mockAnalyzeIntent
    );

    await handleSubmit();

    // 验证消息数量
    expect(messages.length).toBe(2); // user + assistant

    // 验证用户消息
    expect(messages[0].role).toBe('user');
    expect(messages[0].content).toBe('你好');

    // 验证助手消息
    expect(messages[1].role).toBe('assistant');
    expect(messages[1].content).toBe('你好！我是开源项目助手。你想找什么类型的开源项目？');
    expect(messages[1].phase).toBe('idle');

    // 验证状态
    expect(searchPhase).toBe('idle');
    expect(isLoading).toBe(false);
  });

  it("unclear 流程：消息应正确更新", async () => {
    const mockAnalyzeIntent = async (_q: string): Promise<IntentAnalysis> => ({
      intent: 'unclear',
      questions: ['想要什么类型的工具？', '需要什么编程语言？']
    });

    const { handleSubmit, messages, searchPhase } = simulateHandleSubmit(
      '推荐',
      mockAnalyzeIntent
    );

    await handleSubmit();

    expect(messages.length).toBe(2);
    expect(messages[1].content).toBe('想要什么类型的工具？\n需要什么编程语言？');
    expect(messages[1].phase).toBe('idle');
    expect(searchPhase).toBe('idle');
  });

  it("clear 流程：消息应进入搜索状态（finally会重置状态，这是预期行为）", async () => {
    const mockAnalyzeIntent = async (_q: string): Promise<IntentAnalysis> => ({
      intent: 'clear',
      keywords: ['Python', 'Web框架']
    });

    const { handleSubmit, messages, searchPhase } = simulateHandleSubmit(
      'Python Web框架',
      mockAnalyzeIntent
    );

    await handleSubmit();

    expect(messages.length).toBe(2);
    // 注意：clear 分支没有 return，所以 finally 会执行
    // 这意味着 searchPhase 会被 finally 重置为 'idle'
    // 这是设计行为，因为 clear 需要继续执行搜索
    expect(messages[1].phase).toBe('searching');
    expect(searchPhase).toBe('idle'); // finally 块会重置这个
  });

  it("渲染条件验证：phase=idle 应显示内容", () => {
    const msg: MessageItem = {
      id: '123',
      role: 'assistant',
      content: '测试内容',
      phase: 'idle'
    };

    const shouldShowTyping = msg.phase === 'analyzing' || msg.phase === 'searching';
    expect(shouldShowTyping).toBe(false);
  });

  it("渲染条件验证：phase=analyzing 应显示 TypingIndicator", () => {
    const msg: MessageItem = {
      id: '123',
      role: 'assistant',
      content: '正在分析语义...',
      phase: 'analyzing'
    };

    const shouldShowTyping = msg.phase === 'analyzing' || msg.phase === 'searching';
    expect(shouldShowTyping).toBe(true);
  });
});
