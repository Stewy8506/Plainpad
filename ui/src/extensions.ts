// Plainpad — JavaScript Extension Runtime & Native Bridge
// Compatible with antinote-extensions-base-v0.0.1.js and Antinote extensions.

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const globalObj: any = typeof window !== 'undefined' ? window : (typeof globalThis !== 'undefined' ? globalThis : {});

// Initialize global registries
if (!globalObj.commandRegistry) {
  globalObj.commandRegistry = [];
}
if (!globalObj.extensionRegistry) {
  globalObj.extensionRegistry = [];
}

// Global active extension context (for scoping callAPI permissions)
let currentCallingExtension: Extension | null = null;

// Preference in-memory cache
const preferencesCache = new Map<string, string>();

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const t = globalObj.__TAURI__;
  if (t) {
    if (t.core && typeof t.core.invoke === 'function') {
      return t.core.invoke(cmd, args) as Promise<T>;
    }
    if (typeof t.invoke === 'function') {
      return t.invoke(cmd, args) as Promise<T>;
    }
  }
  const internals = globalObj.__TAURI_INTERNALS__;
  if (internals && typeof internals.invoke === 'function') {
    return internals.invoke(cmd, args) as Promise<T>;
  }
  return null as unknown as T;
}

/**
 * Evaluates a mathematical expression safely without full script execution.
 */
export function evaluateMathExpression(expression: string): { success: boolean; value?: number; error?: string } {
  try {
    if (!expression || typeof expression !== 'string') {
      return { success: false, error: 'Empty math expression' };
    }
    const trimmed = expression.trim();
    if (trimmed.length === 0) {
      return { success: false, error: 'Empty math expression' };
    }

    // Security check: disallow identifiers other than Math.*
    if (/(window|document|global|process|eval|Function|require|import|this|prototype|constructor|class)/i.test(trimmed)) {
      return { success: false, error: 'Disallowed identifiers in math expression' };
    }

    // Replace power operator
    const cleaned = trimmed.replace(/\^/g, '**');

    // Verify safe characters: numbers, math operators, parentheses, commas, whitespace, and Math methods
    const withoutMath = cleaned.replace(/Math\.[a-zA-Z_]+/g, '');
    if (!/^[0-9+\-*/().%\s,eE]+$/.test(withoutMath)) {
      return { success: false, error: 'Invalid characters in math expression' };
    }

    const fn = new Function(`"use strict"; return (${cleaned});`);
    const val = fn();
    if (typeof val === 'number' && !isNaN(val)) {
      return { success: true, value: val };
    }
    return { success: false, error: 'Expression did not evaluate to a valid number' };
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    return { success: false, error: msg };
  }
}

/**
 * MathEvaluator bridge compatible with Antinote base extensions.
 */
export const MathEvaluator = {
  eval(expression: string): number {
    const fn = globalObj.__evaluateMathExpression || evaluateMathExpression;
    const result = fn(expression);
    if (!result || !result.success) {
      throw new Error((result && result.error) || 'Math evaluation failed');
    }
    return result.value as number;
  },

  evalSafe(expression: string, defaultValue = 0): number {
    try {
      return this.eval(expression);
    } catch {
      return defaultValue;
    }
  },

  isMathExpression(str: unknown): boolean {
    if (typeof str !== 'string') return false;
    return /[+\-*/()^%]/.test(str) && !/^\s*-?\d+\.?\d*\s*$/.test(str);
  },

  parseNumeric(value: unknown): number {
    if (typeof value === 'number') return value;
    if (typeof value !== 'string') return parseFloat(String(value)) || 0;
    if (this.isMathExpression(value)) {
      return this.eval(value);
    }
    return parseFloat(value) || 0;
  },
};

/**
 * Preference class representing an extension configuration item.
 */
export class Preference {
  key: string;
  label: string;
  type: 'bool' | 'string' | 'selectOne' | 'selectMultiple';
  defaultValue: unknown;
  options: string[] | null;
  helpText: string | null;

  constructor(
    configOrKey: unknown,
    label?: string,
    type?: 'bool' | 'string' | 'selectOne' | 'selectMultiple',
    defaultValue?: unknown,
    options?: string[] | null,
    helpText?: string | null
  ) {
    if (typeof configOrKey === 'object' && configOrKey !== null) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const cfg = configOrKey as any;
      this.key = String(cfg.key);
      this.label = String(cfg.label || cfg.title || cfg.key);
      this.type = cfg.type || 'string';
      this.defaultValue = cfg.defaultValue;
      this.options = Array.isArray(cfg.options) ? cfg.options : null;
      this.helpText = cfg.helpText || cfg.description || null;
    } else {
      this.key = String(configOrKey);
      this.label = label || String(configOrKey);
      this.type = type || 'string';
      this.defaultValue = defaultValue;
      this.options = options || null;
      this.helpText = helpText || null;
    }
  }
}

/**
 * Parameter definition for command invocation.
 */
export class Parameter {
  parameterType: string;
  name: string;
  helpText: string;
  defaultValue: unknown;
  required: boolean;

  constructor(
    configOrType: unknown,
    name?: string,
    helpText?: string,
    defaultValue?: unknown,
    required?: boolean
  ) {
    if (typeof configOrType === 'object' && configOrType !== null) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const cfg = configOrType as any;
      this.parameterType = String(cfg.type || cfg.parameterType || 'string');
      this.name = String(cfg.name || '');
      this.helpText = String(cfg.helpText || cfg.description || 'This parameter needs help text.');
      this.defaultValue = cfg.default !== undefined ? cfg.default : (cfg.defaultValue !== undefined ? cfg.defaultValue : null);
      this.required = cfg.required !== undefined ? Boolean(cfg.required) : true;
    } else {
      this.parameterType = String(configOrType);
      this.name = name || '';
      this.helpText = helpText || 'This parameter needs help text.';
      this.defaultValue = defaultValue !== undefined ? defaultValue : null;
      this.required = required !== undefined ? Boolean(required) : true;
    }
  }
}

/**
 * Tutorial / example command for autocompletion and help.
 */
export class TutorialCommand {
  command: string;
  description: string;

  constructor(configOrCmd: unknown, description?: string) {
    if (typeof configOrCmd === 'object' && configOrCmd !== null) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const cfg = configOrCmd as any;
      this.command = String(cfg.command || '');
      this.description = String(cfg.description || '');
    } else {
      this.command = String(configOrCmd || '');
      this.description = description || '';
    }
  }
}

/**
 * Standard return object for extension commands.
 */
export class ReturnObject {
  status: 'success' | 'error';
  message: string;
  payload: unknown;

  constructor(configOrStatus: unknown, message?: string, payload?: unknown) {
    if (typeof configOrStatus === 'object' && configOrStatus !== null) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const cfg = configOrStatus as any;
      this.status = cfg.status === 'success' ? 'success' : 'error';
      this.message = String(cfg.message || 'Undefined error.');
      this.payload = cfg.payload !== undefined ? cfg.payload : (cfg.data !== undefined ? cfg.data : '');
    } else {
      this.status = configOrStatus === 'success' ? 'success' : 'error';
      this.message = message || 'Undefined error.';
      this.payload = payload !== undefined ? payload : '';
    }
  }
}

/**
 * Extension root defining permissions, endpoints, and credentials required.
 */
export class Extension {
  name: string;
  version: string;
  commands: Command[] = [];
  endpoints: string[];
  requiredAPIKeys: string[];
  preferences: Preference[] = [];
  author: string;
  category: string;
  dataScope: 'none' | 'line' | 'full';
  dependencies: string[];
  isService: boolean;

  constructor(
    configOrName: unknown,
    version?: string,
    endpoints?: string[],
    requiredAPIKeys?: string[],
    author?: string,
    category?: string,
    dataScope?: 'none' | 'line' | 'full'
  ) {
    if (typeof configOrName === 'object' && configOrName !== null) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const cfg = configOrName as any;
      this.name = String(cfg.name || '');
      this.version = String(cfg.version || '1.0.0');
      this.endpoints = Array.isArray(cfg.endpoints) ? cfg.endpoints : [];
      this.requiredAPIKeys = Array.isArray(cfg.requiredAPIKeys) ? cfg.requiredAPIKeys : [];
      this.author = String(cfg.author || '');
      this.category = String(cfg.category || 'Utilities');
      this.dataScope = cfg.dataScope || 'full';
      this.dependencies = Array.isArray(cfg.dependencies) ? cfg.dependencies : [];
      this.isService = Boolean(cfg.isService);
    } else {
      this.name = String(configOrName || '');
      this.version = String(version || '1.0.0');
      this.endpoints = Array.isArray(endpoints) ? endpoints : [];
      this.requiredAPIKeys = Array.isArray(requiredAPIKeys) ? requiredAPIKeys : [];
      this.author = String(author || '');
      this.category = String(category || 'Utilities');
      this.dataScope = dataScope || 'full';
      this.dependencies = [];
      this.isService = false;
    }

    if (Array.isArray(globalObj.extensionRegistry)) {
      globalObj.extensionRegistry.push(this);
    }
  }

  register_command(cmd: Command): void {
    if (!this.commands.includes(cmd)) {
      this.commands.push(cmd);
    }
  }

  register_preference(pref: Preference): void {
    if (!this.preferences.includes(pref)) {
      this.preferences.push(pref);
    }
  }
}

/**
 * Command constructor and execution engine.
 */
export class Command {
  name: string;
  parameters: Parameter[];
  extension?: Extension;
  type: 'insert' | 'replaceAll' | 'replaceLine' | 'openURL';
  helpText: string;
  tutorials: TutorialCommand[];
  aliases: string[];
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  execute: (payload: any) => any;
  commandRegistry: Command[];

  constructor(
    configOrName: unknown,
    parameters?: Parameter[],
    type?: 'insert' | 'replaceAll' | 'replaceLine' | 'openURL',
    helpText?: string,
    tutorials?: TutorialCommand[],
    extension?: Extension
  ) {
    if (typeof configOrName === 'object' && configOrName !== null) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const cfg = configOrName as any;
      this.name = String(cfg.name || '');
      this.parameters = Array.isArray(cfg.parameters) ? cfg.parameters : [];
      this.extension = cfg.extension;
      this.type = cfg.type || 'replaceLine';
      this.helpText = String(cfg.helpText || 'This command replaces text in a line.');
      this.tutorials = Array.isArray(cfg.tutorials) ? cfg.tutorials : [];
      this.aliases = Array.isArray(cfg.aliases) ? cfg.aliases : [];
      if (typeof cfg.execute === 'function') {
        this.execute = cfg.execute;
      } else {
        this.execute = () => 'Hello, World!';
      }
    } else {
      this.name = String(configOrName || '');
      this.parameters = Array.isArray(parameters) ? parameters : [];
      this.type = type || 'replaceLine';
      this.helpText = String(helpText || 'This command replaces text in a line.');
      this.tutorials = Array.isArray(tutorials) ? tutorials : [];
      this.aliases = [];
      this.extension = extension;
      this.execute = () => 'Hello, World!';
    }

    if (this.extension && typeof this.extension.register_command === 'function') {
      this.extension.register_command(this);
    }

    this.commandRegistry = globalObj.commandRegistry;
    if (Array.isArray(globalObj.commandRegistry)) {
      globalObj.commandRegistry.push(this);
    }
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  getParsedParams(payload: any): any[] {
    const resolved: unknown[] = [];
    const rawParams = payload?.parameters || [];

    for (let i = 0; i < this.parameters.length; i++) {
      const param = this.parameters[i];
      let rawValue = rawParams[i];

      const isStringType = param.parameterType === 'string' || param.parameterType === 'paragraph';
      const isMissing = rawValue === undefined || rawValue === null || (!isStringType && rawValue === '');

      if (param.required && isMissing) {
        throw new Error(`Required parameter '${param.name}' is missing`);
      }

      if (isMissing) {
        rawValue = param.defaultValue;
      }

      let parsedValue: unknown;
      switch (param.parameterType) {
        case 'float':
          parsedValue = parseFloat(String(rawValue));
          break;
        case 'int':
          parsedValue = parseInt(String(rawValue), 10);
          break;
        case 'expression':
        case 'mathExpression':
          parsedValue = MathEvaluator.parseNumeric(rawValue);
          break;
        case 'bool':
          if (typeof rawValue === 'boolean') {
            parsedValue = rawValue;
          } else {
            parsedValue = String(rawValue).toLowerCase() === 'true';
          }
          break;
        case 'string':
        default:
          parsedValue = rawValue !== undefined && rawValue !== null ? String(rawValue) : '';
          break;
      }

      resolved.push(parsedValue);
    }

    return resolved;
  }
}

/**
 * Native bridge for secure network requests.
 * Replaces {{API_KEY}} placeholders securely inside native code.
 */
export async function callAPI(
  apiKeyName: string,
  url: string,
  method: string,
  headers: string | Record<string, string>,
  body?: string
): Promise<{ success: boolean; statusCode?: number; data?: string; error?: string }> {
  try {
    const headersStr = typeof headers === 'string' ? headers : JSON.stringify(headers || {});

    // Collect approved endpoints from active extension context or registered extensions
    let approvedEndpoints: string[] = [];
    if (currentCallingExtension && currentCallingExtension.endpoints) {
      approvedEndpoints = currentCallingExtension.endpoints;
    } else {
      const exts = (globalObj.extensionRegistry as Extension[]) || [];
      for (const ext of exts) {
        if (ext.endpoints && ext.endpoints.length > 0) {
          approvedEndpoints.push(...ext.endpoints);
        }
      }
    }

    const res = await invoke<{
      success: boolean;
      statusCode?: number;
      status_code?: number;
      data?: string;
      error?: string;
    }>('call_api', {
      apiKeyName,
      url,
      method,
      headers: headersStr,
      body: body || null,
      approvedEndpoints,
    });

    if (!res) {
      return { success: false, error: 'Empty response from call_api' };
    }

    return {
      success: res.success,
      statusCode: res.statusCode !== undefined ? res.statusCode : res.status_code,
      data: res.data,
      error: res.error,
    };
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    return {
      success: false,
      error: msg,
    };
  }
}

/**
 * Native bridge to read an extension preference.
 */
export function getExtensionPreference(extensionName: string, key: string): string | null {
  const cacheKey = `${extensionName.trim()}_${key.trim()}`;
  if (preferencesCache.has(cacheKey)) {
    return preferencesCache.get(cacheKey) || null;
  }
  return null;
}

/**
 * Native bridge to persist an extension preference.
 */
export async function setExtensionPreference(
  extensionName: string,
  key: string,
  value: string
): Promise<void> {
  const cacheKey = `${extensionName.trim()}_${key.trim()}`;
  preferencesCache.set(cacheKey, value);
  await invoke('set_extension_preference', {
    extensionName,
    key,
    value,
  }).catch(console.error);
}

/**
 * Pre-processes extension scripts to ensure asynchronous callAPI operations
 * work seamlessly inside webviews without blocking the main event loop.
 */
function preprocessExtensionScript(script: string): string {
  // If script contains callAPI without await, prepend await and mark enclosing function async
  let transformed = script.replace(/(?<!await\s+)callAPI\s*\(/g, 'await callAPI(');
  // Transform function declarations/expressions that use await callAPI into async functions
  transformed = transformed.replace(
    /function\s*(\w*)\s*\(([^)]*)\)\s*\{([^}]*await\s+callAPI)/g,
    'async function $1($2) {$3'
  );
  return transformed;
}

/**
 * Extension runtime manager coordinating extension lifecycle, sandbox execution,
 * and command dispatch.
 */
export class ExtensionRuntime {
  private initialized = false;

  /**
   * Initializes runtime global objects and exposes bridge methods.
   */
  async init(): Promise<void> {
    if (this.initialized) return;

    globalObj.Extension = Extension;
    globalObj.Command = Command;
    globalObj.Preference = Preference;
    globalObj.Parameter = Parameter;
    globalObj.TutorialCommand = TutorialCommand;
    globalObj.ReturnObject = ReturnObject;
    globalObj.MathEvaluator = MathEvaluator;
    globalObj.callAPI = callAPI;
    globalObj.getExtensionPreference = getExtensionPreference;
    globalObj.setExtensionPreference = setExtensionPreference;
    globalObj.__evaluateMathExpression = evaluateMathExpression;

    await this.refreshPreferences();
    this.registerSampleOpenAiExtension();
    this.initialized = true;
  }

  /**
   * Refreshes the in-memory preference cache from database settings.
   */
  async refreshPreferences(): Promise<void> {
    try {
      const settings = (await invoke<Record<string, unknown>>('get_settings')) || {};
      for (const [key, val] of Object.entries(settings)) {
        if (key.startsWith('ext_pref_')) {
          const rest = key.substring('ext_pref_'.length);
          preferencesCache.set(rest, String(val));
        }
      }
    } catch {
      // Preferences will use defaults
    }
  }

  /**
   * Registers and evaluates an extension script in a controlled sandbox.
   */
  async registerExtension(extensionScript: string): Promise<Extension | null> {
    await this.init();

    const previousExtensions = [...((globalObj.extensionRegistry as Extension[]) || [])];
    const processedScript = preprocessExtensionScript(extensionScript);

    try {
      const runner = new Function(
        'window',
        'global',
        'Extension',
        'Command',
        'Preference',
        'Parameter',
        'TutorialCommand',
        'ReturnObject',
        'MathEvaluator',
        'callAPI',
        'getExtensionPreference',
        'setExtensionPreference',
        `"use strict";\nreturn (async function() {\n${processedScript}\n})();`
      );

      await runner(
        globalObj,
        globalObj,
        Extension,
        Command,
        Preference,
        Parameter,
        TutorialCommand,
        ReturnObject,
        MathEvaluator,
        callAPI,
        getExtensionPreference,
        setExtensionPreference
      );

      const currentExtensions = (globalObj.extensionRegistry as Extension[]) || [];
      const newExt = currentExtensions.find((e) => !previousExtensions.includes(e));
      return newExt || null;
    } catch (err: unknown) {
      console.error('Failed to register extension script:', err);
      throw err;
    }
  }

  /**
   * Returns all registered commands.
   */
  getCommands(): Command[] {
    return (globalObj.commandRegistry as Command[]) || [];
  }

  /**
   * Finds a command by name or alias.
   */
  findCommand(name: string): Command | undefined {
    const q = name.trim().toLowerCase();
    const commands = this.getCommands();
    return commands.find(
      (cmd) =>
        cmd.name.toLowerCase() === q ||
        (Array.isArray(cmd.aliases) && cmd.aliases.some((a) => a.toLowerCase() === q))
    );
  }

  /**
   * Builds execution payload filtered strictly according to declared DataScope.
   */
  buildPayload(
    command: Command,
    rawParams: unknown[],
    currentLine: string,
    fullNote: string
  ): { line: string; note: string; parameters: unknown[] } {
    const scope = command.extension?.dataScope || 'full';
    let line = '';
    let note = '';

    switch (scope) {
      case 'none':
        line = '';
        note = '';
        break;
      case 'line':
        line = currentLine;
        note = '';
        break;
      case 'full':
      default:
        line = currentLine;
        note = fullNote;
        break;
    }

    return {
      line,
      note,
      parameters: rawParams,
    };
  }

  /**
   * Executes a command by name with provided arguments and note context.
   */
  async executeCommand(
    name: string,
    rawParams: unknown[],
    currentLine: string,
    fullNote: string
  ): Promise<ReturnObject> {
    await this.init();

    const command = this.findCommand(name);
    if (!command) {
      return new ReturnObject('error', `Command '${name}' not found.`);
    }

    const payload = this.buildPayload(command, rawParams, currentLine, fullNote);

    try {
      currentCallingExtension = command.extension || null;
      const rawResult = await Promise.resolve(command.execute(payload));

      if (rawResult instanceof ReturnObject) {
        return rawResult;
      }
      if (typeof rawResult === 'object' && rawResult !== null && 'status' in rawResult) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const anyRes = rawResult as any;
        return new ReturnObject(anyRes.status, anyRes.message, anyRes.payload || anyRes.data);
      }
      return new ReturnObject('success', 'Execution succeeded', rawResult);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      return new ReturnObject('error', `Extension execution failed: ${msg}`);
    } finally {
      currentCallingExtension = null;
    }
  }

  /**
   * Registers the built-in sample OpenAI extension matching sample-openai-extension.js.
   */
  registerSampleOpenAiExtension(): Extension {
    const existing = ((globalObj.extensionRegistry as Extension[]) || []).find((e) => e.name === 'openai');
    if (existing) return existing;

    const extensionName = 'openai';
    const extensionRoot = new Extension(
      extensionName,
      '1.0.0',
      ['https://api.openai.com/v1/chat/completions'],
      ['apikey_openai'],
      'anthropics',
      'AI',
      'line'
    );

    const modelPref = new Preference(
      'model',
      'OpenAI Model',
      'selectOne',
      'gpt-4o',
      ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo', 'gpt-3.5-turbo'],
      'Select which OpenAI model to use for completions'
    );
    extensionRoot.register_preference(modelPref);

    const systemPromptPref = new Preference(
      'systemPrompt',
      'System Prompt',
      'string',
      'You are a helpful assistant integrated into a plaintext scratch notes app. Be concise and direct.',
      null,
      'Customize the system prompt sent to OpenAI'
    );
    extensionRoot.register_preference(systemPromptPref);

    const cacheKey = `${extensionName}_model`;
    if (!preferencesCache.has(cacheKey)) {
      preferencesCache.set(cacheKey, 'gpt-4o');
    }

    const gpt = new Command(
      'gpt',
      [
        new Parameter('string', 'prompt', 'The prompt to send to GPT', 'Hello, how are you?'),
        new Parameter('int', 'max_tokens', 'Maximum tokens in the response', 150),
        new Parameter('float', 'temperature', 'Randomness of the response (0.0-2.0)', 0.7),
      ],
      'insert',
      "Ask GPT a question using OpenAI's API",
      [
        new TutorialCommand('gpt(What is the capital of France?)', 'Ask GPT a simple question'),
        new TutorialCommand('gpt(Explain quantum computing in simple terms, 300)', 'Get a longer explanation'),
        new TutorialCommand('gpt(Write a haiku about coding, 100, 1.2)', 'More creative response'),
      ],
      extensionRoot
    );

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    gpt.execute = async function (payload: any) {
      try {
        const [prompt, max_tokens, temperature] = this.getParsedParams(payload);

        if (!prompt || String(prompt).trim() === '') {
          return new ReturnObject('error', 'Please provide a prompt.');
        }

        const maxTokensNum = Number(max_tokens);
        if (isNaN(maxTokensNum) || maxTokensNum < 1 || maxTokensNum > 4000) {
          return new ReturnObject('error', 'max_tokens must be between 1 and 4000.');
        }

        const tempNum = Number(temperature);
        if (isNaN(tempNum) || tempNum < 0 || tempNum > 2) {
          return new ReturnObject('error', 'temperature must be between 0.0 and 2.0.');
        }

        const model = getExtensionPreference(extensionName, 'model') || 'gpt-4o';
        const systemPrompt =
          getExtensionPreference(extensionName, 'systemPrompt') ||
          'You are a helpful assistant integrated into a plaintext scratch notes app. Be concise and direct.';

        const url = 'https://api.openai.com/v1/chat/completions';
        const method = 'POST';

        const headers = JSON.stringify({
          'Content-Type': 'application/json',
          'Authorization': 'Bearer {{API_KEY}}',
        });

        const body = JSON.stringify({
          model,
          messages: [
            { role: 'system', content: systemPrompt },
            { role: 'user', content: String(prompt) },
          ],
          max_tokens: maxTokensNum,
          temperature: tempNum,
        });

        const result = await callAPI('apikey_openai', url, method, headers, body);

        if (!result.success) {
          return new ReturnObject('error', 'API call failed: ' + (result.error || 'Unknown error'));
        }

        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        let responseData: any = {};
        if (typeof result.data === 'string') {
          try {
            responseData = JSON.parse(result.data);
          } catch {
            responseData = {};
          }
        } else if (result.data) {
          responseData = result.data;
        }

        if (responseData.error) {
          return new ReturnObject(
            'error',
            'OpenAI API error: ' + (responseData.error.message || JSON.stringify(responseData.error))
          );
        }

        const responseText = responseData.choices?.[0]?.message?.content?.trim() || '';
        return new ReturnObject('success', 'GPT responded successfully', responseText);
      } catch (error: unknown) {
        const msg = error instanceof Error ? error.message : String(error);
        return new ReturnObject('error', 'Extension error: ' + msg);
      }
    };

    return extensionRoot;
  }
}

// Global runtime singleton
export const extensionRuntime = new ExtensionRuntime();

export interface AiModel {
  id: string;
  name: string;
  provider: string;
  context_length?: number;
}

/**
 * Fetches models from OpenAI, Anthropic, or local Ollama.
 */
export async function fetchAiModels(provider: string): Promise<AiModel[]> {
  const models = await invoke<AiModel[]>('fetch_ai_models', { provider });
  return models || [];
}

/**
 * Parses comma-separated arguments supporting single/double quotes with commas inside.
 */
export function parseArguments(argsStr: string): unknown[] {
  const trimmed = argsStr.trim();
  if (trimmed.length === 0) return [];

  const params: unknown[] = [];
  let current = '';
  let inDoubleQuote = false;
  let inSingleQuote = false;
  let depth = 0;

  for (let i = 0; i < trimmed.length; i++) {
    const ch = trimmed[i];
    const prev = i > 0 ? trimmed[i - 1] : '';

    if (ch === '"' && prev !== '\\' && !inSingleQuote) {
      inDoubleQuote = !inDoubleQuote;
      current += ch;
    } else if (ch === "'" && prev !== '\\' && !inDoubleQuote) {
      inSingleQuote = !inSingleQuote;
      current += ch;
    } else if ((ch === '(' || ch === '[' || ch === '{') && !inDoubleQuote && !inSingleQuote) {
      depth++;
      current += ch;
    } else if ((ch === ')' || ch === ']' || ch === '}') && !inDoubleQuote && !inSingleQuote) {
      depth--;
      current += ch;
    } else if (ch === ',' && !inDoubleQuote && !inSingleQuote && depth === 0) {
      params.push(cleanParam(current));
      current = '';
    } else {
      current += ch;
    }
  }

  if (current.trim().length > 0) {
    params.push(cleanParam(current));
  }

  return params;
}

function cleanParam(raw: string): unknown {
  const t = raw.trim();
  if (
    (t.startsWith('"') && t.endsWith('"') && t.length >= 2) ||
    (t.startsWith("'") && t.endsWith("'") && t.length >= 2)
  ) {
    return t.substring(1, t.length - 1).replace(/\\(["'])/g, '$1');
  }
  if (t === 'true') return true;
  if (t === 'false') return false;
  if (t === 'null') return null;
  if (t === 'undefined') return undefined;
  if (/^-?\d+(\.\d+)?$/.test(t)) {
    const num = Number(t);
    if (!isNaN(num)) return num;
  }
  return t;
}

/**
 * Parses double-colon invocation syntax `::commandName(...)` or alias invocation `gpt(...)`.
 */
export function parseCommandInvocation(
  text: string
): { commandName: string; rawParams: unknown[] } | null {
  const trimmed = text.trim();
  if (!trimmed) return null;

  // 1. Matches ::commandName(...) or bare ::commandName
  const doubleColonMatch = trimmed.match(/^::([a-zA-Z0-9_-]+)(?:\((.*)\))?$/s);
  if (doubleColonMatch) {
    const commandName = doubleColonMatch[1];
    const rawArgsStr = doubleColonMatch[2] !== undefined ? doubleColonMatch[2] : '';
    const rawParams = parseArguments(rawArgsStr);
    return { commandName, rawParams };
  }

  // 2. Direct command alias invocation like gpt(...)
  const directMatch = trimmed.match(/^([a-zA-Z0-9_-]+)\((.*)\)$/s);
  if (directMatch) {
    const name = directMatch[1];
    const cmd = extensionRuntime.findCommand(name);
    if (cmd) {
      const rawParams = parseArguments(directMatch[2] || '');
      return { commandName: cmd.name, rawParams };
    }
  }

  return null;
}

/**
 * Top-level execution helper for extension commands.
 */
export async function executeCommand(
  name: string,
  rawParams: unknown[],
  currentLine: string,
  fullNote: string
): Promise<ReturnObject> {
  return extensionRuntime.executeCommand(name, rawParams, currentLine, fullNote);
}

export interface DetectedCommand {
  invocation: string;
  startIndex: number;
  endIndex: number;
  commandName: string;
  rawParams: unknown[];
}

/**
 * Searches a line of text for a command invocation (either ::cmd(...) or cmd(...)).
 */
export function findCommandInLine(lineText: string): DetectedCommand | null {
  const trimmed = lineText.trim();
  if (!trimmed) return null;

  // 1. Check if line contains double colon ::
  const doubleColonIdx = lineText.lastIndexOf('::');
  if (doubleColonIdx !== -1) {
    const candidate = lineText.substring(doubleColonIdx);
    const parsed = parseCommandInvocation(candidate);
    if (parsed) {
      return {
        invocation: candidate.trim(),
        startIndex: doubleColonIdx,
        endIndex: doubleColonIdx + candidate.trimEnd().length,
        commandName: parsed.commandName,
        rawParams: parsed.rawParams,
      };
    }
  }

  // 2. Check full trimmed line for direct command alias like gpt(...)
  const parsed = parseCommandInvocation(trimmed);
  if (parsed) {
    const startIdx = lineText.indexOf(trimmed);
    return {
      invocation: trimmed,
      startIndex: startIdx,
      endIndex: startIdx + trimmed.length,
      commandName: parsed.commandName,
      rawParams: parsed.rawParams,
    };
  }

  return null;
}
