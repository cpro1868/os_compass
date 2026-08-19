import { describe, it, expect } from "vitest";

interface IntentAnalysis {
  intent: string;
  intent_type?: string;
  keywords?: string[];
  questions?: string[];
  options?: string[];
  guidance?: string;
}

type SearchPhase = 'idle' | 'analyzing' | 'searching';

interface MessageItem {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  phase?: SearchPhase;
  results?: any;
  clarification?: IntentAnalysis;
}

describe("意图搜索 off_topic 处理逻辑", () => {
  
  function shouldShowTypingIndicator(msg: MessageItem): boolean {
    return msg.phase === 'analyzing' || msg.phase === 'searching';
  }

  function updatePhase(messages: MessageItem[], loadingId: string, phase: SearchPhase, content: string): MessageItem[] {
    return messages.map(msg => {
      return msg.id === loadingId ? { ...msg, phase, content } : msg;
    });
  }

  it("off_topic 意图应正确设置 phase 为 idle", () => {
    const intent: IntentAnalysis = {
      intent: 'off_topic',
      guidance: '你好！我是开源项目助手。你想找什么类型的开源项目？'
    };

    const loadingId = '123';
    let messages: MessageItem[] = [
      { id: '123', role: 'assistant', content: '正在分析语义...', phase: 'analyzing' }
    ];

    if (intent.intent === 'off_topic' && intent.guidance) {
      messages = updatePhase(messages, loadingId, 'idle', intent.guidance);
    }

    const updatedMsg = messages.find(m => m.id === loadingId);
    expect(updatedMsg?.phase).toBe('idle');
    expect(updatedMsg?.content).toBe('你好！我是开源项目助手。你想找什么类型的开源项目？');
  });

  it("phase=idle 时应显示消息内容而非 TypingIndicator", () => {
    const msg: MessageItem = {
      id: '123',
      role: 'assistant',
      content: '你好！我是开源项目助手。',
      phase: 'idle'
    };

    const showTyping = shouldShowTypingIndicator(msg);
    expect(showTyping).toBe(false);
  });

  it("phase=analyzing 时应显示 TypingIndicator", () => {
    const msg: MessageItem = {
      id: '123',
      role: 'assistant',
      content: '正在分析语义...',
      phase: 'analyzing'
    };

    const showTyping = shouldShowTypingIndicator(msg);
    expect(showTyping).toBe(true);
  });

  it("phase=searching 时应显示 TypingIndicator", () => {
    const msg: MessageItem = {
      id: '123',
      role: 'assistant',
      content: '正在搜索...',
      phase: 'searching'
    };

    const showTyping = shouldShowTypingIndicator(msg);
    expect(showTyping).toBe(true);
  });

  it("phase=undefined 时应显示消息内容", () => {
    const msg: MessageItem = {
      id: '123',
      role: 'assistant',
      content: '这是消息内容',
      phase: undefined
    };

    const showTyping = shouldShowTypingIndicator(msg);
    expect(showTyping).toBe(false);
  });

  it("完整流程：off_topic 意图从 analyzing 到 idle", () => {
    const intent: IntentAnalysis = {
      intent: 'off_topic',
      guidance: '你好！我是开源项目助手。你想找什么类型的开源项目？'
    };

    const loadingId = '123';
    
    // 初始状态：正在分析
    let messages: MessageItem[] = [
      { id: '123', role: 'assistant', content: '正在分析语义...', phase: 'analyzing' }
    ];

    expect(shouldShowTypingIndicator(messages[0])).toBe(true);

    // 处理 off_topic 意图
    if (intent.intent === 'off_topic' && intent.guidance) {
      messages = updatePhase(messages, loadingId, 'idle', intent.guidance);
    }

    // 验证更新后的状态
    const updatedMsg = messages.find(m => m.id === loadingId);
    expect(updatedMsg?.phase).toBe('idle');
    expect(updatedMsg?.content).toBe('你好！我是开源项目助手。你想找什么类型的开源项目？');
    expect(shouldShowTypingIndicator(updatedMsg!)).toBe(false);
  });

  it("unclear 意图应正确设置 phase 为 idle", () => {
    const intent: IntentAnalysis = {
      intent: 'unclear',
      questions: ['想要什么类型的工具？', '需要什么编程语言？']
    };

    const loadingId = '123';
    let messages: MessageItem[] = [
      { id: '123', role: 'assistant', content: '正在分析语义...', phase: 'analyzing' }
    ];

    if (intent.intent === 'unclear' && intent.questions) {
      messages = updatePhase(messages, loadingId, 'idle', intent.questions.join('\n'));
    }

    const updatedMsg = messages.find(m => m.id === loadingId);
    expect(updatedMsg?.phase).toBe('idle');
    expect(updatedMsg?.content).toBe('想要什么类型的工具？\n需要什么编程语言？');
  });

  it("clear 意图应设置 phase 为 searching", () => {
    const loadingId = '123';
    let messages: MessageItem[] = [
      { id: '123', role: 'assistant', content: '正在分析语义...', phase: 'analyzing' }
    ];

    // clear 意图需要调用搜索，所以 phase 变为 searching
    messages = updatePhase(messages, loadingId, 'searching', '正在搜索本地项目和联网查询...');

    const updatedMsg = messages.find(m => m.id === loadingId);
    expect(updatedMsg?.phase).toBe('searching');
  });
});
