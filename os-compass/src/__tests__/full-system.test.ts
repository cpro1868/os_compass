import { describe, it, expect, beforeEach } from "vitest";

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

interface SearchState {
  messages: MessageItem[];
  query: string;
  isLoading: boolean;
  searchPhase: SearchPhase;
  conversationId: string | null;
  loadingIdRef: string | null;
}

function createInitialState(): SearchState {
  return {
    messages: [],
    query: '',
    isLoading: false,
    searchPhase: 'idle',
    conversationId: null,
    loadingIdRef: null,
  };
}

function simulateAnalyzeIntent(userInput: string): IntentAnalysis {
  const input = userInput.toLowerCase().trim();
  
  if (input === '你好' || input === 'hi' || input === 'hello') {
    return {
      intent: 'off_topic',
      guidance: '你好！我是开源罗盘助手。你想找什么类型的开源项目？'
    };
  }
  
  if (input === '推荐' || input === '推荐一些') {
    return {
      intent: 'unclear',
      questions: ['想要什么类型的工具？', '需要什么编程语言？', '有什么特定需求吗？'],
      options: ['开发框架', '命令行工具', 'GUI应用', 'Web开发', '移动应用']
    };
  }
  
  if (input.includes('python') || input.includes('web') || input.includes('rust')) {
    return {
      intent: 'clear',
      keywords: userInput.split(' ').filter(w => w.length > 0)
    };
  }
  
  return {
    intent: 'clear',
    keywords: [userInput]
  };
}

function updatePhase(state: SearchState, phase: SearchPhase, content: string): SearchState {
  const newState = { ...state };
  newState.searchPhase = phase;
  
  if (newState.loadingIdRef) {
    newState.messages = newState.messages.map(msg => {
      return msg.id === newState.loadingIdRef 
        ? { ...msg, phase, content } 
        : msg;
    });
  }
  
  return newState;
}

function handleSubmit(state: SearchState, input: string): SearchState {
  const newState = { ...state };
  
  if (!input.trim() || newState.isLoading) {
    return state;
  }
  
  const userMessage: MessageItem = {
    id: `user-${Date.now()}`,
    role: 'user',
    content: input,
  };
  
  const loadingId = `loading-${Date.now()}`;
  newState.loadingIdRef = loadingId;
  
  const loadingMessage: MessageItem = {
    id: loadingId,
    role: 'assistant',
    content: '正在分析语义...',
    phase: 'analyzing',
  };
  
  newState.messages = [...newState.messages, userMessage, loadingMessage];
  newState.query = '';
  newState.isLoading = true;
  newState.searchPhase = 'analyzing';
  
  const intent = simulateAnalyzeIntent(input);
  
  if (intent.intent === 'off_topic' && intent.guidance) {
    newState.searchPhase = 'idle';
    newState.isLoading = false;
    newState.messages = newState.messages.map(msg => {
      return msg.id === loadingId
        ? { ...msg, phase: 'idle', content: intent.guidance! }
        : msg;
    });
    newState.loadingIdRef = null;
    return newState;
  }
  
  if (intent.intent === 'unclear' && intent.questions) {
    newState.searchPhase = 'idle';
    newState.isLoading = false;
    newState.messages = newState.messages.map(msg => {
      return msg.id === loadingId
        ? { ...msg, phase: 'idle', content: intent.questions!.join('\n'), clarification: intent }
        : msg;
    });
    newState.loadingIdRef = null;
    return newState;
  }
  
  if (intent.intent === 'clear') {
    newState.searchPhase = 'searching';
    newState.messages = newState.messages.map(msg => {
      return msg.id === loadingId
        ? { ...msg, phase: 'searching', content: '正在搜索本地项目和联网查询...' }
        : msg;
    });
  }
  
  return newState;
}

function shouldShowTypingIndicator(msg: MessageItem): boolean {
  return msg.phase === 'analyzing' || msg.phase === 'searching';
}

describe("意图搜索完整系统测试", () => {
  let state: SearchState;
  
  beforeEach(() => {
    state = createInitialState();
  });
  
  describe("1. 状态初始化测试", () => {
    it("初始状态应为 idle", () => {
      expect(state.searchPhase).toBe('idle');
      expect(state.isLoading).toBe(false);
      expect(state.messages.length).toBe(0);
    });
  });
  
  describe("2. off_topic 意图测试", () => {
    it("输入'你好'应返回 off_topic 意图", () => {
      const intent = simulateAnalyzeIntent('你好');
      expect(intent.intent).toBe('off_topic');
      expect(intent.guidance).toBeDefined();
      expect(intent.guidance!.length).toBeGreaterThan(0);
    });
    
    it("off_topic 流程：状态应正确转换", () => {
      state = handleSubmit(state, '你好');
      
      expect(state.searchPhase).toBe('idle');
      expect(state.isLoading).toBe(false);
      expect(state.messages.length).toBe(2);
      expect(state.messages[0].role).toBe('user');
      expect(state.messages[0].content).toBe('你好');
      expect(state.messages[1].role).toBe('assistant');
      expect(state.messages[1].phase).toBe('idle');
      expect(state.messages[1].content).toContain('开源');
    });
    
    it("off_topic 流程：渲染应显示消息内容", () => {
      state = handleSubmit(state, '你好');
      const assistantMsg = state.messages[1];
      
      expect(shouldShowTypingIndicator(assistantMsg)).toBe(false);
      expect(assistantMsg.content).toBeDefined();
    });
    
    it("多个 off_topic 输入应独立处理", () => {
      state = handleSubmit(state, '你好');
      state = handleSubmit(state, 'hi');
      
      expect(state.messages.length).toBe(4);
      expect(state.messages[3].content).toContain('开源');
    });
  });
  
  describe("3. unclear 意图测试", () => {
    it("输入'推荐'应返回 unclear 意图", () => {
      const intent = simulateAnalyzeIntent('推荐');
      expect(intent.intent).toBe('unclear');
      expect(intent.questions).toBeDefined();
      expect(intent.questions!.length).toBeGreaterThan(0);
    });
    
    it("unclear 流程：状态应正确转换", () => {
      state = handleSubmit(state, '推荐');
      
      expect(state.searchPhase).toBe('idle');
      expect(state.isLoading).toBe(false);
      expect(state.messages.length).toBe(2);
      expect(state.messages[1].content).toContain('\n');
      expect(state.messages[1].clarification).toBeDefined();
    });
    
    it("unclear 流程：渲染应显示问题列表", () => {
      state = handleSubmit(state, '推荐');
      const assistantMsg = state.messages[1];
      
      expect(shouldShowTypingIndicator(assistantMsg)).toBe(false);
      expect(assistantMsg.content).toContain('想要什么类型');
    });
  });
  
  describe("4. clear 意图测试", () => {
    it("输入'Python Web'应返回 clear 意图", () => {
      const intent = simulateAnalyzeIntent('Python Web');
      expect(intent.intent).toBe('clear');
      expect(intent.keywords).toBeDefined();
      expect(intent.keywords!.length).toBeGreaterThan(0);
    });
    
    it("clear 流程：状态应进入 searching", () => {
      state = handleSubmit(state, 'Python Web框架');
      
      expect(state.searchPhase).toBe('searching');
      expect(state.messages[1].phase).toBe('searching');
      expect(state.messages[1].content).toContain('搜索');
    });
    
    it("clear 流程：渲染应显示 TypingIndicator", () => {
      state = handleSubmit(state, 'Rust Web');
      const assistantMsg = state.messages[1];
      
      expect(shouldShowTypingIndicator(assistantMsg)).toBe(true);
    });
  });
  
  describe("5. 渲染逻辑边界测试", () => {
    it("phase=idle 应显示内容", () => {
      const msg: MessageItem = { id: '1', role: 'assistant', content: '测试', phase: 'idle' };
      expect(shouldShowTypingIndicator(msg)).toBe(false);
    });
    
    it("phase=analyzing 应显示 TypingIndicator", () => {
      const msg: MessageItem = { id: '1', role: 'assistant', content: '正在分析', phase: 'analyzing' };
      expect(shouldShowTypingIndicator(msg)).toBe(true);
    });
    
    it("phase=searching 应显示 TypingIndicator", () => {
      const msg: MessageItem = { id: '1', role: 'assistant', content: '正在搜索', phase: 'searching' };
      expect(shouldShowTypingIndicator(msg)).toBe(true);
    });
    
    it("phase=undefined 应显示内容", () => {
      const msg: MessageItem = { id: '1', role: 'assistant', content: '测试' };
      expect(shouldShowTypingIndicator(msg)).toBe(false);
    });
    
    it("phase=null 应显示内容", () => {
      const msg: MessageItem = { id: '1', role: 'assistant', content: '测试', phase: undefined };
      expect(shouldShowTypingIndicator(msg)).toBe(false);
    });
  });
  
  describe("6. 完整对话流程测试", () => {
    it("完整流程：用户 → off_topic → 再次查询 clear", () => {
      state = handleSubmit(state, '你好');
      expect(state.messages[1].content).toContain('开源');
      
      state = handleSubmit(state, 'Python Web框架');
      expect(state.searchPhase).toBe('searching');
    });
    
    it("完整流程：用户 → unclear → clear", () => {
      state = handleSubmit(state, '推荐');
      expect(state.messages[1].clarification).toBeDefined();
      
      state = handleSubmit(state, 'Rust CLI工具');
      expect(state.searchPhase).toBe('searching');
    });
    
    it("完整流程：多个用户消息", () => {
      state = handleSubmit(state, '你好');
      state = handleSubmit(state, '推荐');
      state = handleSubmit(state, 'Python Web');
      
      expect(state.messages.filter(m => m.role === 'user').length).toBe(3);
      expect(state.messages.filter(m => m.role === 'assistant').length).toBe(3);
    });
  });
  
  describe("7. 错误处理测试", () => {
    it("空输入应被拒绝", () => {
      const newState = handleSubmit(state, '');
      expect(newState).toBe(state);
    });
    
    it("仅空格输入应被拒绝", () => {
      const newState = handleSubmit(state, '   ');
      expect(newState).toBe(state);
    });
    
    it("loading 状态下输入应被拒绝", () => {
      state.isLoading = true;
      const newState = handleSubmit(state, 'test');
      expect(newState).toBe(state);
    });
  });
  
  describe("8. updatePhase 函数测试", () => {
    it("updatePhase 应正确更新状态", () => {
      state.messages = [{ id: 'test', role: 'assistant', content: 'old', phase: 'analyzing' }];
      state.loadingIdRef = 'test';
      
      state = updatePhase(state, 'idle', 'new content');
      
      expect(state.searchPhase).toBe('idle');
      expect(state.messages[0].content).toBe('new content');
      expect(state.messages[0].phase).toBe('idle');
    });
    
    it("updatePhase 不应影响其他消息", () => {
      state.messages = [
        { id: 'msg1', role: 'user', content: 'user msg' },
        { id: 'msg2', role: 'assistant', content: 'old', phase: 'analyzing' },
      ];
      state.loadingIdRef = 'msg2';
      
      state = updatePhase(state, 'idle', 'updated');
      
      expect(state.messages[0].content).toBe('user msg');
      expect(state.messages[1].content).toBe('updated');
    });
  });
  
  describe("9. 意图分类器测试", () => {
    it("问候语应识别为 off_topic", () => {
      const greetings = ['你好', 'hi', 'hello', 'HI', 'Hello'];
      greetings.forEach(g => {
        const intent = simulateAnalyzeIntent(g);
        expect(intent.intent).toBe('off_topic');
      });
    });
    
    it("推荐请求应识别为 unclear", () => {
      const intents = simulateAnalyzeIntent('推荐');
      expect(intents.intent).toBe('unclear');
    });
    
    it("包含关键词的输入应识别为 clear", () => {
      const inputs = ['Python', 'web开发', 'Rust项目'];
      inputs.forEach(input => {
        const intent = simulateAnalyzeIntent(input);
        expect(intent.intent).toBe('clear');
        expect(intent.keywords).toBeDefined();
      });
    });
  });
  
  describe("10. 端到端流程测试", () => {
    it("完整用户体验：off_topic 引导", () => {
      const query = '你好';
      const intent = simulateAnalyzeIntent(query);
      
      expect(intent.intent).toBe('off_topic');
      
      state = handleSubmit(state, query);
      
      expect(state.messages.length).toBe(2);
      expect(state.messages[1].content).toContain('开源');
      expect(state.messages[1].phase).toBe('idle');
      expect(shouldShowTypingIndicator(state.messages[1])).toBe(false);
    });
    
    it("完整用户体验：unclear 追问", () => {
      const query = '推荐一些';
      const intent = simulateAnalyzeIntent(query);
      
      expect(intent.intent).toBe('unclear');
      
      state = handleSubmit(state, query);
      
      expect(state.messages[1].content).toContain('想要什么类型');
      expect(state.messages[1].clarification).toBeDefined();
    });
    
    it("完整用户体验：clear 搜索", () => {
      const query = 'Python Web框架';
      const intent = simulateAnalyzeIntent(query);
      
      expect(intent.intent).toBe('clear');
      
      state = handleSubmit(state, query);
      
      expect(state.messages[1].phase).toBe('searching');
      expect(shouldShowTypingIndicator(state.messages[1])).toBe(true);
    });
  });
});
