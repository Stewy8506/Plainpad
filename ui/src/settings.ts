// Plainpad — Dedicated Preferences Window
export {};

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const win = window as any;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const t = win.__TAURI__;
  if (t) {
    if (t.core && typeof t.core.invoke === 'function') {
      return t.core.invoke(cmd, args) as Promise<T>;
    }
    if (typeof t.invoke === 'function') {
      return t.invoke(cmd, args) as Promise<T>;
    }
  }
  const internals = win.__TAURI_INTERNALS__;
  if (internals && typeof internals.invoke === 'function') {
    return internals.invoke(cmd, args) as Promise<T>;
  }
  console.warn(`[mock] invoke(${cmd})`, args);
  return null as unknown as T;
}

interface RatesInfo {
  count: number;
  last_updated_unix: number;
  is_seed: boolean;
}

interface TriggerConfigDTO {
  disableAllKeywords: boolean;
  allowArgumentsAfterKeyword: boolean;
  sumTriggers: string[];
  avgTriggers: string[];
  mathTriggers: string[];
  countTriggers: string[];
  listTriggers: string[];
  checkTriggers: string[];
  codeTriggers: string[];
  pasteTriggers: string[];
  timerTriggers: string[];
  customKeywordLabel?: string;
  customKeywordDescription?: string;
}

// ---------- DOM Elements ----------

const tabButtons = document.querySelectorAll<HTMLButtonElement>('.tab-btn');
const tabPanels = document.querySelectorAll<HTMLElement>('.tab-panel');

// General
const chkAutostart = document.getElementById('setting-autostart') as HTMLInputElement;
const chkStartNewNote = document.getElementById('setting-start-new-note') as HTMLInputElement;
const selTrayMode = document.getElementById('setting-tray-mode') as HTMLSelectElement;
const chkHideOnBlur = document.getElementById('setting-hide-on-blur') as HTMLInputElement;
const chkRestorePin = document.getElementById('setting-restore-pin') as HTMLInputElement;
const chkSwipeNav = document.getElementById('setting-swipe-nav') as HTMLInputElement;
const rngSwipeSensitivity = document.getElementById('setting-swipe-sensitivity') as HTMLInputElement;
const badgeSwipeSensitivity = document.getElementById('swipe-sensitivity-val') as HTMLElement;

// Appearance
const selTheme = document.getElementById('setting-theme') as HTMLSelectElement;
const selFontFamily = document.getElementById('setting-font-family') as HTMLSelectElement;
const rngFontSize = document.getElementById('setting-font-size') as HTMLInputElement;
const badgeFontSize = document.getElementById('font-size-val') as HTMLElement;
const rngLineHeight = document.getElementById('setting-line-height') as HTMLInputElement;
const badgeLineHeight = document.getElementById('line-height-val') as HTMLElement;
const rngDimming = document.getElementById('setting-dimming') as HTMLInputElement;
const badgeDimming = document.getElementById('dimming-val') as HTMLElement;
const rngWindowOpacity = document.getElementById('setting-window-opacity') as HTMLInputElement;
const badgeWindowOpacity = document.getElementById('window-opacity-val') as HTMLElement;
const selGridOverlay = document.getElementById('setting-grid-overlay') as HTMLSelectElement | null;
const chkSolidBars = document.getElementById('setting-solid-bars') as HTMLInputElement | null;
const inputGlobalHotkey = document.getElementById('setting-global-hotkey') as HTMLInputElement | null;
const btnSaveHotkey = document.getElementById('btn-save-hotkey') as HTMLButtonElement | null;

// Editor
const chkDefaultPlain = document.getElementById('setting-default-plain') as HTMLInputElement;
const chkGlobalPlain = document.getElementById('setting-global-plain') as HTMLInputElement;
const selTabWidth = document.getElementById('setting-tab-width') as HTMLSelectElement;
const chkAutoBrackets = document.getElementById('setting-auto-brackets') as HTMLInputElement;
const chkListCheckboxes = document.getElementById('setting-list-checkboxes') as HTMLInputElement;
const chkInlineBullets = document.getElementById('setting-inline-bullets') as HTMLInputElement;
const chkInlineNumbers = document.getElementById('setting-inline-numbers') as HTMLInputElement;
const chkDisableLinkShortening = document.getElementById('setting-disable-link-shortening') as HTMLInputElement;
const chkDisableLinkHighlighting = document.getElementById('setting-disable-link-highlighting') as HTMLInputElement;
const chkActivateVimMode = document.getElementById('setting-activate-vim-mode') as HTMLInputElement;
const rngCaretNudge = document.getElementById('setting-caret-nudge') as HTMLInputElement;
const badgeCaretNudge = document.getElementById('caret-nudge-val') as HTMLElement;
const rngCaretNudgeList = document.getElementById('setting-caret-nudge-list') as HTMLInputElement;
const badgeCaretNudgeList = document.getElementById('caret-nudge-list-val') as HTMLElement;
const chkForceRTL = document.getElementById('setting-force-rtl') as HTMLInputElement;
const chkReadOnly = document.getElementById('setting-read-only') as HTMLInputElement;

// Keywords & Triggers
const chkDisableAllKeywords = document.getElementById('setting-disable-all-keywords') as HTMLInputElement;
const chkAllowArgsAfterKeyword = document.getElementById('setting-allow-args-after-keyword') as HTMLInputElement;
const inputTriggersSum = document.getElementById('setting-triggers-sum') as HTMLInputElement;
const inputTriggersAvg = document.getElementById('setting-triggers-avg') as HTMLInputElement;
const inputTriggersMath = document.getElementById('setting-triggers-math') as HTMLInputElement;
const inputTriggersCount = document.getElementById('setting-triggers-count') as HTMLInputElement;
const inputTriggersList = document.getElementById('setting-triggers-list') as HTMLInputElement;
const inputTriggersCheck = document.getElementById('setting-triggers-check') as HTMLInputElement;
const inputTriggersCode = document.getElementById('setting-triggers-code') as HTMLInputElement;
const inputTriggersPaste = document.getElementById('setting-triggers-paste') as HTMLInputElement;
const inputTriggersTimer = document.getElementById('setting-triggers-timer') as HTMLInputElement;

const chkDetectMath = document.getElementById('setting-detect-math') as HTMLInputElement;
const chkDetectTimer = document.getElementById('setting-detect-timer') as HTMLInputElement;
const chkDetectList = document.getElementById('setting-detect-list') as HTMLInputElement;
const chkDetectCode = document.getElementById('setting-detect-code') as HTMLInputElement;
const selDefaultCurrency = document.getElementById('setting-default-currency') as HTMLSelectElement;
const selNumberFormat = document.getElementById('setting-number-format') as HTMLSelectElement;
const selCalcPrecision = document.getElementById('setting-calc-precision') as HTMLSelectElement;

// Timer
const selDefaultTimerDuration = document.getElementById('setting-default-timer-duration') as HTMLSelectElement;
const chkTimerNotification = document.getElementById('setting-timer-notification') as HTMLInputElement;
const chkTimerSound = document.getElementById('setting-timer-sound') as HTMLInputElement;
const chkTimerPomoFullscreen = document.getElementById('setting-timer-pomo-fullscreen') as HTMLInputElement;
const chkTimerPauseOnClose = document.getElementById('setting-timer-pause-on-close') as HTMLInputElement;
const chkTimerAutohide = document.getElementById('setting-timer-autohide') as HTMLInputElement;

// Currency & Rates
const ratesCountBadge = document.getElementById('rates-count-badge') as HTMLElement;
const ratesStatusPill = document.getElementById('rates-status-pill') as HTMLElement;
const ratesLastUpdated = document.getElementById('rates-last-updated') as HTMLElement;
const btnRefreshRates = document.getElementById('btn-refresh-rates') as HTMLButtonElement;
const chkRatesAutoupdate = document.getElementById('setting-rates-autoupdate') as HTMLInputElement;
const chkIncludeCrypto = document.getElementById('setting-include-crypto') as HTMLInputElement;

// Storage
const storagePathDisplay = document.getElementById('storage-path-display') as HTMLElement;
const btnOpenDataDir = document.getElementById('btn-open-data-dir') as HTMLButtonElement;
const selExportFormat = document.getElementById('setting-export-format') as HTMLSelectElement;
const selTrashRetention = document.getElementById('setting-trash-retention') as HTMLSelectElement;
const btnEmptyTrash = document.getElementById('btn-empty-trash') as HTMLButtonElement;

// Extensions & AI Providers
interface ApiKeyMeta {
  key_name: string;
  is_configured: boolean;
  masked_preview: string;
}

interface AiModel {
  id: string;
  name: string;
  provider: string;
  context_length?: number;
}

const keyInputOpenai = document.getElementById('key-input-openai') as HTMLInputElement | null;
const btnToggleOpenai = document.getElementById('btn-toggle-openai') as HTMLButtonElement | null;
const btnSaveOpenai = document.getElementById('btn-save-openai') as HTMLButtonElement | null;
const btnClearOpenai = document.getElementById('btn-clear-openai') as HTMLButtonElement | null;
const badgeStatusOpenai = document.getElementById('badge-status-openai') as HTMLElement | null;

const keyInputAnthropic = document.getElementById('key-input-anthropic') as HTMLInputElement | null;
const btnToggleAnthropic = document.getElementById('btn-toggle-anthropic') as HTMLButtonElement | null;
const btnSaveAnthropic = document.getElementById('btn-save-anthropic') as HTMLButtonElement | null;
const btnClearAnthropic = document.getElementById('btn-clear-anthropic') as HTMLButtonElement | null;
const badgeStatusAnthropic = document.getElementById('badge-status-anthropic') as HTMLElement | null;

const hostInputOllama = document.getElementById('host-input-ollama') as HTMLInputElement | null;
const btnTestOllama = document.getElementById('btn-test-ollama') as HTMLButtonElement | null;
const badgeStatusOllama = document.getElementById('badge-status-ollama') as HTMLElement | null;

const selAiDefaultModel = document.getElementById('setting-ai-default-model') as HTMLSelectElement | null;
const btnRefreshModels = document.getElementById('btn-refresh-models') as HTMLButtonElement | null;

const selExtPrefOpenaiModel = document.getElementById('ext-pref-openai-model') as HTMLSelectElement | null;
const txtExtPrefOpenaiSystemPrompt = document.getElementById('ext-pref-openai-systemPrompt') as HTMLTextAreaElement | null;

// Export
const chkExportFirstLineTitle = document.getElementById('setting-export-first-line-title') as HTMLInputElement | null;
const chkExportOmitKeywords = document.getElementById('setting-export-omit-keywords') as HTMLInputElement | null;
const chkExportPlainTextEnabled = document.getElementById('setting-export-plain-text-enabled') as HTMLInputElement | null;
const chkExportMarkdownEnabled = document.getElementById('setting-export-markdown-enabled') as HTMLInputElement | null;
const chkExportObsidianEnabled = document.getElementById('setting-export-obsidian-enabled') as HTMLInputElement | null;
const txtExportObsidianVault = document.getElementById('setting-export-obsidian-vault') as HTMLInputElement | null;
const chkExportObsidianConvertMarkdown = document.getElementById('setting-export-obsidian-convert-markdown') as HTMLInputElement | null;
const chkExportBearEnabled = document.getElementById('setting-export-bear-enabled') as HTMLInputElement | null;
const chkExportBearConvertMarkdown = document.getElementById('setting-export-bear-convert-markdown') as HTMLInputElement | null;
const chkExportAppleNotesEnabled = document.getElementById('setting-export-apple-notes-enabled') as HTMLInputElement | null;
const chkExportAppleNotesRichText = document.getElementById('setting-export-apple-notes-rich-text') as HTMLInputElement | null;
const chkExportNotionEnabled = document.getElementById('setting-export-notion-enabled') as HTMLInputElement | null;
const chkExportNotionSaveDatabase = document.getElementById('setting-export-notion-save-database') as HTMLInputElement | null;
const txtExportNotionDatabaseName = document.getElementById('setting-export-notion-database-name') as HTMLInputElement | null;
const chkExportCustomEnabled = document.getElementById('setting-export-custom-enabled') as HTMLInputElement | null;
const txtExportCustomName = document.getElementById('setting-export-custom-name') as HTMLInputElement | null;
const txtExportCustomURLTemplate = document.getElementById('setting-export-custom-url-template') as HTMLInputElement | null;
const btnSettingsExportZip = document.getElementById('btn-settings-export-zip') as HTMLButtonElement | null;

// ---------- Tab Switching ----------

function setupTabs(): void {
  tabButtons.forEach((btn) => {
    btn.addEventListener('click', () => {
      const targetTab = btn.dataset.tab;
      if (!targetTab) return;

      tabButtons.forEach((b) => {
        b.classList.remove('active');
        b.setAttribute('aria-selected', 'false');
      });
      tabPanels.forEach((p) => p.classList.remove('active'));

      btn.classList.add('active');
      btn.setAttribute('aria-selected', 'true');

      const targetPanel = document.getElementById(`panel-${targetTab}`);
      if (targetPanel) {
        targetPanel.classList.add('active');
      }
    });
  });
}

// ---------- Save Helper ----------

async function saveSetting(key: string, value: unknown): Promise<void> {
  await invoke('set_setting', { key, value });
}

// ---------- Load & Wire Settings ----------

async function loadSettings(): Promise<void> {
  const settings = (await invoke<Record<string, unknown>>('get_settings')) || {};

  // General
  const autostart = await invoke<boolean>('is_autostart_enabled').catch(() => false);
  if (chkAutostart) chkAutostart.checked = !!autostart;

  if (chkStartNewNote && settings.start_new_note !== undefined) {
    chkStartNewNote.checked = !!settings.start_new_note;
  }
  if (selTrayMode && settings.tray_mode !== undefined) {
    selTrayMode.value = String(settings.tray_mode);
  }
  if (chkHideOnBlur && settings.hide_on_blur !== undefined) {
    chkHideOnBlur.checked = !!settings.hide_on_blur;
  }
  if (chkRestorePin) {
    if (settings.restorePinOnOpen !== undefined) {
      chkRestorePin.checked = !!settings.restorePinOnOpen;
    } else if (settings.restore_pin !== undefined) {
      chkRestorePin.checked = !!settings.restore_pin;
    }
  }
  if (chkSwipeNav && settings.swipe_nav !== undefined) {
    chkSwipeNav.checked = !!settings.swipe_nav;
  }
  if (rngSwipeSensitivity && settings.swipe_sensitivity !== undefined) {
    rngSwipeSensitivity.value = String(settings.swipe_sensitivity);
    if (badgeSwipeSensitivity) badgeSwipeSensitivity.textContent = String(settings.swipe_sensitivity);
  }

  // Appearance
  const theme = String(settings.theme || 'dark');
  if (selTheme) selTheme.value = theme;
  applyTheme(theme);

  if (selFontFamily && settings.font_family !== undefined) {
    selFontFamily.value = String(settings.font_family);
  }
  if (rngFontSize && settings.font_size !== undefined) {
    rngFontSize.value = String(settings.font_size);
    if (badgeFontSize) badgeFontSize.textContent = `${settings.font_size}px`;
  }
  if (rngLineHeight && settings.line_height !== undefined) {
    rngLineHeight.value = String(settings.line_height);
    if (badgeLineHeight) badgeLineHeight.textContent = String(settings.line_height);
  }
  if (rngDimming && settings.dimming !== undefined) {
    rngDimming.value = String(settings.dimming);
    if (badgeDimming) badgeDimming.textContent = `${settings.dimming}%`;
  }
  if (rngWindowOpacity && settings.window_opacity !== undefined) {
    rngWindowOpacity.value = String(settings.window_opacity);
    if (badgeWindowOpacity) badgeWindowOpacity.textContent = `${settings.window_opacity}%`;
  }

  // Canvas & Window Chrome
  if (selGridOverlay) {
    const gridMode = String(settings.gridOverlayMode || settings.grid_overlay || 'none');
    selGridOverlay.value = gridMode;
  }
  if (chkSolidBars) {
    const solid = settings.solidBarsInTransparencyMode !== undefined
      ? !!settings.solidBarsInTransparencyMode
      : (settings.solid_bars !== undefined ? !!settings.solid_bars : (settings._solidBarsInTransparencyMode !== undefined ? !!settings._solidBarsInTransparencyMode : false));
    chkSolidBars.checked = solid;
  }

  // Global Hotkey
  if (inputGlobalHotkey) {
    const hotkey = String(settings.globalHotkey || settings.global_hotkey || 'Ctrl+Space');
    inputGlobalHotkey.value = hotkey;
  }

  // Editor
  if (chkDefaultPlain && settings.default_plain !== undefined) {
    chkDefaultPlain.checked = !!settings.default_plain;
  }
  if (chkGlobalPlain && settings.global_plain !== undefined) {
    chkGlobalPlain.checked = !!settings.global_plain;
  }
  if (selTabWidth && settings.tab_width !== undefined) {
    selTabWidth.value = String(settings.tab_width);
  }
  if (chkAutoBrackets && settings.auto_brackets !== undefined) {
    chkAutoBrackets.checked = !!settings.auto_brackets;
  }
  if (chkListCheckboxes) {
    if (settings.inlineChecklistCheckboxesEnabled !== undefined) {
      chkListCheckboxes.checked = !!settings.inlineChecklistCheckboxesEnabled;
    } else if (settings.list_checkboxes !== undefined) {
      chkListCheckboxes.checked = !!settings.list_checkboxes;
    }
  }
  if (chkInlineBullets && settings.inlineChecklistBulletsEnabled !== undefined) {
    chkInlineBullets.checked = !!settings.inlineChecklistBulletsEnabled;
  }
  if (chkInlineNumbers && settings.inlineNumberedListsEnabled !== undefined) {
    chkInlineNumbers.checked = !!settings.inlineNumberedListsEnabled;
  }
  if (chkDisableLinkShortening && settings.disableLinkShortening !== undefined) {
    chkDisableLinkShortening.checked = !!settings.disableLinkShortening;
  }
  if (chkDisableLinkHighlighting && settings.disableLinkHighlighting !== undefined) {
    chkDisableLinkHighlighting.checked = !!settings.disableLinkHighlighting;
  }
  if (chkActivateVimMode && settings.activateVimMode !== undefined) {
    chkActivateVimMode.checked = !!settings.activateVimMode;
  }
  if (rngCaretNudge) {
    const val = settings._caretVerticalNudgeRaw !== undefined
      ? Number(settings._caretVerticalNudgeRaw)
      : (settings.caret_vertical_nudge !== undefined ? Number(settings.caret_vertical_nudge) : 0);
    rngCaretNudge.value = String(val);
    if (badgeCaretNudge) badgeCaretNudge.textContent = `${val}px`;
  }
  if (rngCaretNudgeList) {
    const val = settings._caretVerticalNudgeForListsRaw !== undefined
      ? Number(settings._caretVerticalNudgeForListsRaw)
      : (settings.caret_vertical_nudge_list !== undefined ? Number(settings.caret_vertical_nudge_list) : 0);
    rngCaretNudgeList.value = String(val);
    if (badgeCaretNudgeList) badgeCaretNudgeList.textContent = `${val}px`;
  }
  if (chkForceRTL) {
    const isRtl = settings.forceRTL !== undefined
      ? !!settings.forceRTL
      : (settings._forceRTLStorage !== undefined ? !!settings._forceRTLStorage : false);
    chkForceRTL.checked = isRtl;
  }
  if (chkReadOnly && settings.read_only !== undefined) {
    chkReadOnly.checked = !!settings.read_only;
  }

  // Keywords & Math
  if (chkDetectMath && settings.detect_math !== undefined) {
    chkDetectMath.checked = !!settings.detect_math;
  }
  if (chkDetectTimer && settings.detect_timer !== undefined) {
    chkDetectTimer.checked = !!settings.detect_timer;
  }
  if (chkDetectList && settings.detect_list !== undefined) {
    chkDetectList.checked = !!settings.detect_list;
  }
  if (chkDetectCode && settings.detect_code !== undefined) {
    chkDetectCode.checked = !!settings.detect_code;
  }
  if (selDefaultCurrency && settings.default_currency !== undefined) {
    selDefaultCurrency.value = String(settings.default_currency);
  }
  if (selNumberFormat && settings.number_format !== undefined) {
    selNumberFormat.value = String(settings.number_format);
  }
  if (selCalcPrecision && settings.calc_precision !== undefined) {
    selCalcPrecision.value = String(settings.calc_precision);
  }

  // Timer
  if (selDefaultTimerDuration && settings.default_timer_duration !== undefined) {
    selDefaultTimerDuration.value = String(settings.default_timer_duration);
  }
  if (chkTimerNotification && settings.timer_notification !== undefined) {
    chkTimerNotification.checked = !!settings.timer_notification;
  }
  if (chkTimerSound) {
    if (settings.timerCountdownPlaySound !== undefined) {
      chkTimerSound.checked = !!settings.timerCountdownPlaySound;
    } else if (settings.timer_sound !== undefined) {
      chkTimerSound.checked = !!settings.timer_sound;
    } else {
      chkTimerSound.checked = true;
    }
  }
  if (chkTimerPomoFullscreen) {
    chkTimerPomoFullscreen.checked = settings.timerPomoShowFullScreenAlert !== undefined
      ? !!settings.timerPomoShowFullScreenAlert
      : false;
  }
  if (chkTimerPauseOnClose) {
    chkTimerPauseOnClose.checked = settings.pauseTheTimerIfAppClosed !== undefined
      ? !!settings.pauseTheTimerIfAppClosed
      : true;
  }
  if (chkTimerAutohide && settings.timer_autohide !== undefined) {
    chkTimerAutohide.checked = !!settings.timer_autohide;
  }

  // Currency & Rates
  if (chkRatesAutoupdate && settings.rates_autoupdate !== undefined) {
    chkRatesAutoupdate.checked = !!settings.rates_autoupdate;
  }
  if (chkIncludeCrypto && settings.include_crypto !== undefined) {
    chkIncludeCrypto.checked = !!settings.include_crypto;
  }

  // Storage
  if (selExportFormat && settings.export_format !== undefined) {
    selExportFormat.value = String(settings.export_format);
  }
  if (selTrashRetention && settings.trash_retention !== undefined) {
    selTrashRetention.value = String(settings.trash_retention);
  }

  // Load live data paths & rates status
  await updateRatesStatus();
  await updateStoragePath();
  await loadTriggerSettings();
  await loadApiKeysAndExtensions(settings);
}

async function loadApiKeysAndExtensions(settings: Record<string, unknown>): Promise<void> {
  try {
    const keys = (await invoke<ApiKeyMeta[]>('list_api_keys')) || [];
    const openaiMeta = keys.find((k) => k.key_name === 'apikey_openai');
    if (badgeStatusOpenai && keyInputOpenai) {
      if (openaiMeta && openaiMeta.is_configured) {
        badgeStatusOpenai.textContent = 'Configured';
        badgeStatusOpenai.className = 'status-badge configured';
        keyInputOpenai.placeholder = openaiMeta.masked_preview || 'sk-...xxxx';
      } else {
        badgeStatusOpenai.textContent = 'Not Set';
        badgeStatusOpenai.className = 'status-badge missing';
        keyInputOpenai.placeholder = 'sk-...';
      }
    }

    const anthropicMeta = keys.find((k) => k.key_name === 'apikey_anthropic');
    if (badgeStatusAnthropic && keyInputAnthropic) {
      if (anthropicMeta && anthropicMeta.is_configured) {
        badgeStatusAnthropic.textContent = 'Configured';
        badgeStatusAnthropic.className = 'status-badge configured';
        keyInputAnthropic.placeholder = anthropicMeta.masked_preview || 'sk-ant-...xxxx';
      } else {
        badgeStatusAnthropic.textContent = 'Not Set';
        badgeStatusAnthropic.className = 'status-badge missing';
        keyInputAnthropic.placeholder = 'sk-ant-...';
      }
    }

    if (hostInputOllama && settings.ollama_host) {
      hostInputOllama.value = String(settings.ollama_host);
    }

    if (selAiDefaultModel && settings.ai_default_model) {
      selAiDefaultModel.value = String(settings.ai_default_model);
    }

    // Load extension preferences
    try {
      const savedModel = await invoke<string | null>('get_extension_preference', {
        extensionName: 'openai',
        key: 'model',
      });
      if (selExtPrefOpenaiModel && savedModel) {
        selExtPrefOpenaiModel.value = savedModel;
      }
    } catch {
      // Ignore
    }

    try {
      const savedPrompt = await invoke<string | null>('get_extension_preference', {
        extensionName: 'openai',
        key: 'systemPrompt',
      });
      if (txtExtPrefOpenaiSystemPrompt && savedPrompt) {
        txtExtPrefOpenaiSystemPrompt.value = savedPrompt;
      }
    } catch {
      // Ignore
    }

    // Export Settings
    if (chkExportFirstLineTitle && settings.exportGeneralUseFirstLineAsTitle !== undefined) {
      chkExportFirstLineTitle.checked = !!settings.exportGeneralUseFirstLineAsTitle;
    }
    if (chkExportOmitKeywords && settings.exportGeneralOmitKeyword !== undefined) {
      chkExportOmitKeywords.checked = !!settings.exportGeneralOmitKeyword;
    }
    if (chkExportPlainTextEnabled && settings.exportPlainTextEnabled !== undefined) {
      chkExportPlainTextEnabled.checked = !!settings.exportPlainTextEnabled;
    }
    if (chkExportMarkdownEnabled && settings.exportMarkdownEnabled !== undefined) {
      chkExportMarkdownEnabled.checked = !!settings.exportMarkdownEnabled;
    }
    if (chkExportObsidianEnabled && settings.exportObsidianEnabled !== undefined) {
      chkExportObsidianEnabled.checked = !!settings.exportObsidianEnabled;
    }
    if (txtExportObsidianVault && settings.exportObsidianVault !== undefined) {
      txtExportObsidianVault.value = String(settings.exportObsidianVault);
    }
    if (chkExportObsidianConvertMarkdown && settings.exportObsidianConvertToMarkdown !== undefined) {
      chkExportObsidianConvertMarkdown.checked = !!settings.exportObsidianConvertToMarkdown;
    }
    if (chkExportBearEnabled && settings.exportBearEnabled !== undefined) {
      chkExportBearEnabled.checked = !!settings.exportBearEnabled;
    }
    if (chkExportBearConvertMarkdown && settings.exportBearConvertToMarkdown !== undefined) {
      chkExportBearConvertMarkdown.checked = !!settings.exportBearConvertToMarkdown;
    }
    if (chkExportAppleNotesEnabled && settings.exportAppleNotesEnabled !== undefined) {
      chkExportAppleNotesEnabled.checked = !!settings.exportAppleNotesEnabled;
    }
    if (chkExportAppleNotesRichText && settings.exportAppleNotesConvertToRichText !== undefined) {
      chkExportAppleNotesRichText.checked = !!settings.exportAppleNotesConvertToRichText;
    }
    if (chkExportNotionEnabled && settings.exportNotionEnabled !== undefined) {
      chkExportNotionEnabled.checked = !!settings.exportNotionEnabled;
    }
    if (chkExportNotionSaveDatabase && settings.exportNotionSaveInDatabase !== undefined) {
      chkExportNotionSaveDatabase.checked = !!settings.exportNotionSaveInDatabase;
    }
    if (txtExportNotionDatabaseName && settings.exportNotionDatabaseName !== undefined) {
      txtExportNotionDatabaseName.value = String(settings.exportNotionDatabaseName);
    }
    if (chkExportCustomEnabled && settings.exportCustomEnabled !== undefined) {
      chkExportCustomEnabled.checked = !!settings.exportCustomEnabled;
    }
    if (txtExportCustomName && settings.exportCustomName !== undefined) {
      txtExportCustomName.value = String(settings.exportCustomName);
    }
    if (txtExportCustomURLTemplate && settings.exportCustomURLTemplate !== undefined) {
      txtExportCustomURLTemplate.value = String(settings.exportCustomURLTemplate);
    }
  } catch (err) {
    console.error('Failed to load API keys or extension preferences:', err);
  }
}

function parseCommaAliases(val: string): string[] {
  return val.split(',').map((s) => s.trim()).filter(Boolean);
}

async function loadTriggerSettings(): Promise<void> {
  try {
    const cfg = await invoke<TriggerConfigDTO>('get_trigger_config');
    if (!cfg) return;
    if (chkDisableAllKeywords) chkDisableAllKeywords.checked = !!cfg.disableAllKeywords;
    if (chkAllowArgsAfterKeyword) chkAllowArgsAfterKeyword.checked = cfg.allowArgumentsAfterKeyword ?? true;
    if (inputTriggersSum && cfg.sumTriggers) inputTriggersSum.value = cfg.sumTriggers.join(', ');
    if (inputTriggersAvg && cfg.avgTriggers) inputTriggersAvg.value = cfg.avgTriggers.join(', ');
    if (inputTriggersMath && cfg.mathTriggers) inputTriggersMath.value = cfg.mathTriggers.join(', ');
    if (inputTriggersCount && cfg.countTriggers) inputTriggersCount.value = cfg.countTriggers.join(', ');
    if (inputTriggersList && cfg.listTriggers) inputTriggersList.value = cfg.listTriggers.join(', ');
    if (inputTriggersCheck && cfg.checkTriggers) inputTriggersCheck.value = cfg.checkTriggers.join(', ');
    if (inputTriggersCode && cfg.codeTriggers) inputTriggersCode.value = cfg.codeTriggers.join(', ');
    if (inputTriggersPaste && cfg.pasteTriggers) inputTriggersPaste.value = cfg.pasteTriggers.join(', ');
    if (inputTriggersTimer && cfg.timerTriggers) inputTriggersTimer.value = cfg.timerTriggers.join(', ');
  } catch (err) {
    console.error('Failed to load trigger settings:', err);
  }
}

async function saveTriggerSettings(): Promise<void> {
  const config: TriggerConfigDTO = {
    disableAllKeywords: chkDisableAllKeywords ? chkDisableAllKeywords.checked : false,
    allowArgumentsAfterKeyword: chkAllowArgsAfterKeyword ? chkAllowArgsAfterKeyword.checked : true,
    sumTriggers: parseCommaAliases(inputTriggersSum?.value || 'sum, total, add'),
    avgTriggers: parseCommaAliases(inputTriggersAvg?.value || 'avg, average'),
    mathTriggers: parseCommaAliases(inputTriggersMath?.value || 'math, convert, model'),
    countTriggers: parseCommaAliases(inputTriggersCount?.value || 'count'),
    listTriggers: parseCommaAliases(inputTriggersList?.value || 'list'),
    checkTriggers: parseCommaAliases(inputTriggersCheck?.value || 'check, todo'),
    codeTriggers: parseCommaAliases(inputTriggersCode?.value || 'code'),
    pasteTriggers: parseCommaAliases(inputTriggersPaste?.value || 'paste'),
    timerTriggers: parseCommaAliases(inputTriggersTimer?.value || 'timer'),
  };
  try {
    await invoke('set_trigger_config', { config });
  } catch (err) {
    console.error('Failed to save trigger settings:', err);
  }
}

function applyTheme(theme: string): void {
  if (theme === 'dark' || theme === 'oled' || theme === 'slate' || theme === 'light') {
    document.documentElement.setAttribute('data-theme', theme);
  } else {
    document.documentElement.removeAttribute('data-theme');
  }
}

async function updateRatesStatus(): Promise<void> {
  try {
    const info = await invoke<RatesInfo>('get_rates_info');
    if (info && ratesCountBadge && ratesStatusPill && ratesLastUpdated) {
      ratesCountBadge.textContent = `${info.count} Currencies & Cryptos`;
      if (info.is_seed) {
        ratesStatusPill.textContent = 'Seed Snapshot';
        ratesStatusPill.style.color = '#c2847a';
        ratesLastUpdated.textContent = 'Offline seed snapshot bundled with app. Rates will auto-update on internet connection.';
      } else {
        ratesStatusPill.textContent = 'Live Synced';
        ratesStatusPill.style.color = '#6a9d94';
        const dateStr = info.last_updated_unix > 0
          ? new Date(info.last_updated_unix * 1000).toLocaleString()
          : 'Recently';
        ratesLastUpdated.textContent = `Last synchronized from exchange API: ${dateStr}`;
      }
    }
  } catch (e) {
    console.error('Failed to get rates info:', e);
  }
}

async function updateStoragePath(): Promise<void> {
  try {
    const dir = await invoke<string>('get_data_dir_path');
    if (dir && storagePathDisplay) {
      storagePathDisplay.textContent = dir;
      storagePathDisplay.title = dir;
    }
  } catch (e) {
    console.error('Failed to get data dir path:', e);
  }
}

// ---------- Event Listeners Wire-up ----------

function wireEvents(): void {
  // Autostart
  chkAutostart?.addEventListener('change', async () => {
    await invoke('set_autostart', { enabled: chkAutostart.checked }).catch(console.error);
  });

  chkStartNewNote?.addEventListener('change', () => saveSetting('start_new_note', chkStartNewNote.checked));
  selTrayMode?.addEventListener('change', () => saveSetting('tray_mode', selTrayMode.value));
  chkHideOnBlur?.addEventListener('change', () => saveSetting('hide_on_blur', chkHideOnBlur.checked));
  chkRestorePin?.addEventListener('change', () => {
    saveSetting('restore_pin', chkRestorePin.checked);
    saveSetting('restorePinOnOpen', chkRestorePin.checked);
  });
  chkSwipeNav?.addEventListener('change', () => saveSetting('swipe_nav', chkSwipeNav.checked));
  rngSwipeSensitivity?.addEventListener('input', () => {
    if (badgeSwipeSensitivity) badgeSwipeSensitivity.textContent = rngSwipeSensitivity.value;
    saveSetting('swipe_sensitivity', parseInt(rngSwipeSensitivity.value, 10));
  });

  // Appearance
  selTheme?.addEventListener('change', () => {
    applyTheme(selTheme.value);
    saveSetting('theme', selTheme.value);
  });
  selFontFamily?.addEventListener('change', () => saveSetting('font_family', selFontFamily.value));
  rngFontSize?.addEventListener('input', () => {
    if (badgeFontSize) badgeFontSize.textContent = `${rngFontSize.value}px`;
    saveSetting('font_size', parseInt(rngFontSize.value, 10));
  });
  rngLineHeight?.addEventListener('input', () => {
    if (badgeLineHeight) badgeLineHeight.textContent = rngLineHeight.value;
    saveSetting('line_height', parseFloat(rngLineHeight.value));
  });
  rngDimming?.addEventListener('input', () => {
    if (badgeDimming) badgeDimming.textContent = `${rngDimming.value}%`;
    saveSetting('dimming', parseInt(rngDimming.value, 10));
  });
  rngWindowOpacity?.addEventListener('input', () => {
    if (badgeWindowOpacity) badgeWindowOpacity.textContent = `${rngWindowOpacity.value}%`;
    saveSetting('window_opacity', parseInt(rngWindowOpacity.value, 10));
  });

  // Canvas & Window Chrome
  selGridOverlay?.addEventListener('change', () => {
    saveSetting('grid_overlay', selGridOverlay.value);
    saveSetting('gridOverlayMode', selGridOverlay.value);
  });
  chkSolidBars?.addEventListener('change', () => {
    saveSetting('solid_bars', chkSolidBars.checked);
    saveSetting('solidBarsInTransparencyMode', chkSolidBars.checked);
    saveSetting('_solidBarsInTransparencyMode', chkSolidBars.checked);
  });

  // Global Hotkey
  const applyGlobalHotkey = async () => {
    if (!inputGlobalHotkey) return;
    const hotkey = inputGlobalHotkey.value.trim() || 'Ctrl+Space';
    try {
      await invoke('register_global_hotkey', { hotkey });
      saveSetting('global_hotkey', hotkey);
      saveSetting('globalHotkey', hotkey);
    } catch (err) {
      console.error('Failed to register global hotkey:', err);
      alert(`Could not set global hotkey '${hotkey}': ${err}`);
    }
  };

  btnSaveHotkey?.addEventListener('click', applyGlobalHotkey);
  inputGlobalHotkey?.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      applyGlobalHotkey();
    }
  });

  // Editor
  chkDefaultPlain?.addEventListener('change', () => saveSetting('default_plain', chkDefaultPlain.checked));
  chkGlobalPlain?.addEventListener('change', () => saveSetting('global_plain', chkGlobalPlain.checked));
  selTabWidth?.addEventListener('change', () => saveSetting('tab_width', selTabWidth.value));
  chkAutoBrackets?.addEventListener('change', () => saveSetting('auto_brackets', chkAutoBrackets.checked));
  chkListCheckboxes?.addEventListener('change', () => {
    saveSetting('inlineChecklistCheckboxesEnabled', chkListCheckboxes.checked);
    saveSetting('list_checkboxes', chkListCheckboxes.checked);
  });
  chkInlineBullets?.addEventListener('change', () => saveSetting('inlineChecklistBulletsEnabled', chkInlineBullets.checked));
  chkInlineNumbers?.addEventListener('change', () => saveSetting('inlineNumberedListsEnabled', chkInlineNumbers.checked));
  chkDisableLinkShortening?.addEventListener('change', () => saveSetting('disableLinkShortening', chkDisableLinkShortening.checked));
  chkDisableLinkHighlighting?.addEventListener('change', () => saveSetting('disableLinkHighlighting', chkDisableLinkHighlighting.checked));
  chkActivateVimMode?.addEventListener('change', () => saveSetting('activateVimMode', chkActivateVimMode.checked));
  rngCaretNudge?.addEventListener('input', () => {
    const val = parseFloat(rngCaretNudge.value) || 0;
    if (badgeCaretNudge) badgeCaretNudge.textContent = `${val}px`;
    saveSetting('_caretVerticalNudgeRaw', val);
    saveSetting('caret_vertical_nudge', val);
  });
  rngCaretNudgeList?.addEventListener('input', () => {
    const val = parseFloat(rngCaretNudgeList.value) || 0;
    if (badgeCaretNudgeList) badgeCaretNudgeList.textContent = `${val}px`;
    saveSetting('_caretVerticalNudgeForListsRaw', val);
    saveSetting('caret_vertical_nudge_list', val);
  });
  chkForceRTL?.addEventListener('change', () => {
    saveSetting('forceRTL', chkForceRTL.checked);
    saveSetting('_forceRTLStorage', chkForceRTL.checked);
  });
  chkReadOnly?.addEventListener('change', () => {
    saveSetting('read_only', chkReadOnly.checked);
  });

  // Trigger Controls & Aliases
  chkDisableAllKeywords?.addEventListener('change', () => saveTriggerSettings());
  chkAllowArgsAfterKeyword?.addEventListener('change', () => saveTriggerSettings());
  inputTriggersSum?.addEventListener('change', () => saveTriggerSettings());
  inputTriggersAvg?.addEventListener('change', () => saveTriggerSettings());
  inputTriggersMath?.addEventListener('change', () => saveTriggerSettings());
  inputTriggersCount?.addEventListener('change', () => saveTriggerSettings());
  inputTriggersList?.addEventListener('change', () => saveTriggerSettings());
  inputTriggersCheck?.addEventListener('change', () => saveTriggerSettings());
  inputTriggersCode?.addEventListener('change', () => saveTriggerSettings());
  inputTriggersPaste?.addEventListener('change', () => saveTriggerSettings());
  inputTriggersTimer?.addEventListener('change', () => saveTriggerSettings());

  // Keywords & Math
  chkDetectMath?.addEventListener('change', () => saveSetting('detect_math', chkDetectMath.checked));
  chkDetectTimer?.addEventListener('change', () => saveSetting('detect_timer', chkDetectTimer.checked));
  chkDetectList?.addEventListener('change', () => saveSetting('detect_list', chkDetectList.checked));
  chkDetectCode?.addEventListener('change', () => saveSetting('detect_code', chkDetectCode.checked));
  selDefaultCurrency?.addEventListener('change', () => saveSetting('default_currency', selDefaultCurrency.value));
  selNumberFormat?.addEventListener('change', () => saveSetting('number_format', selNumberFormat.value));
  selCalcPrecision?.addEventListener('change', () => saveSetting('calc_precision', selCalcPrecision.value));

  // Timer
  selDefaultTimerDuration?.addEventListener('change', () => saveSetting('default_timer_duration', parseInt(selDefaultTimerDuration.value, 10)));
  chkTimerNotification?.addEventListener('change', () => saveSetting('timer_notification', chkTimerNotification.checked));
  chkTimerSound?.addEventListener('change', () => {
    saveSetting('timerCountdownPlaySound', chkTimerSound.checked);
    saveSetting('timer_sound', chkTimerSound.checked);
  });
  chkTimerPomoFullscreen?.addEventListener('change', () => saveSetting('timerPomoShowFullScreenAlert', chkTimerPomoFullscreen.checked));
  chkTimerPauseOnClose?.addEventListener('change', () => saveSetting('pauseTheTimerIfAppClosed', chkTimerPauseOnClose.checked));
  chkTimerAutohide?.addEventListener('change', () => saveSetting('timer_autohide', chkTimerAutohide.checked));

  // Currency & Rates
  chkRatesAutoupdate?.addEventListener('change', () => saveSetting('rates_autoupdate', chkRatesAutoupdate.checked));
  chkIncludeCrypto?.addEventListener('change', () => saveSetting('include_crypto', chkIncludeCrypto.checked));

  btnRefreshRates?.addEventListener('click', async () => {
    btnRefreshRates.disabled = true;
    const originalText = btnRefreshRates.innerHTML;
    btnRefreshRates.innerHTML = `<span>Updating…</span>`;
    try {
      await invoke('refresh_rates_now');
      await updateRatesStatus();
      btnRefreshRates.innerHTML = `<span>Updated ✓</span>`;
    } catch (e) {
      btnRefreshRates.innerHTML = `<span>Failed</span>`;
      console.error(e);
    }
    setTimeout(() => {
      btnRefreshRates.innerHTML = originalText;
      btnRefreshRates.disabled = false;
    }, 2000);
  });

  // Storage
  btnOpenDataDir?.addEventListener('click', () => {
    invoke('open_data_dir').catch(console.error);
  });

  selExportFormat?.addEventListener('change', () => saveSetting('export_format', selExportFormat.value));
  selTrashRetention?.addEventListener('change', () => saveSetting('trash_retention', selTrashRetention.value));

  btnEmptyTrash?.addEventListener('click', async () => {
    btnEmptyTrash.disabled = true;
    const originalText = btnEmptyTrash.textContent;
    btnEmptyTrash.textContent = 'Purging…';
    try {
      await invoke('empty_trash_now');
      btnEmptyTrash.textContent = 'Trash Emptied ✓';
    } catch (e) {
      btnEmptyTrash.textContent = 'Error';
      console.error(e);
    }
    setTimeout(() => {
      btnEmptyTrash.textContent = originalText;
      btnEmptyTrash.disabled = false;
    }, 2000);
  });

  wireExtensionEvents();
}

function wireExtensionEvents(): void {
  // Toggle password visibility
  btnToggleOpenai?.addEventListener('click', () => {
    if (keyInputOpenai) {
      keyInputOpenai.type = keyInputOpenai.type === 'password' ? 'text' : 'password';
    }
  });

  btnToggleAnthropic?.addEventListener('click', () => {
    if (keyInputAnthropic) {
      keyInputAnthropic.type = keyInputAnthropic.type === 'password' ? 'text' : 'password';
    }
  });

  // Save OpenAI Key
  btnSaveOpenai?.addEventListener('click', async () => {
    const val = keyInputOpenai?.value.trim();
    if (!val) return;
    try {
      await invoke('store_api_key', { keyName: 'apikey_openai', keyValue: val });
      if (keyInputOpenai) keyInputOpenai.value = '';
      const settings = (await invoke<Record<string, unknown>>('get_settings')) || {};
      await loadApiKeysAndExtensions(settings);
    } catch (err) {
      console.error('Failed to save OpenAI API key:', err);
    }
  });

  // Clear OpenAI Key
  btnClearOpenai?.addEventListener('click', async () => {
    try {
      await invoke('delete_api_key', { keyName: 'apikey_openai' });
      if (keyInputOpenai) keyInputOpenai.value = '';
      const settings = (await invoke<Record<string, unknown>>('get_settings')) || {};
      await loadApiKeysAndExtensions(settings);
    } catch (err) {
      console.error('Failed to clear OpenAI API key:', err);
    }
  });

  // Save Anthropic Key
  btnSaveAnthropic?.addEventListener('click', async () => {
    const val = keyInputAnthropic?.value.trim();
    if (!val) return;
    try {
      await invoke('store_api_key', { keyName: 'apikey_anthropic', keyValue: val });
      if (keyInputAnthropic) keyInputAnthropic.value = '';
      const settings = (await invoke<Record<string, unknown>>('get_settings')) || {};
      await loadApiKeysAndExtensions(settings);
    } catch (err) {
      console.error('Failed to save Anthropic API key:', err);
    }
  });

  // Clear Anthropic Key
  btnClearAnthropic?.addEventListener('click', async () => {
    try {
      await invoke('delete_api_key', { keyName: 'apikey_anthropic' });
      if (keyInputAnthropic) keyInputAnthropic.value = '';
      const settings = (await invoke<Record<string, unknown>>('get_settings')) || {};
      await loadApiKeysAndExtensions(settings);
    } catch (err) {
      console.error('Failed to clear Anthropic API key:', err);
    }
  });

  // Test Ollama Connection
  btnTestOllama?.addEventListener('click', async () => {
    if (!hostInputOllama || !badgeStatusOllama) return;
    const host = hostInputOllama.value.trim() || 'http://localhost:11434';
    await invoke('set_setting', { key: 'ollama_host', value: host });

    badgeStatusOllama.textContent = 'Checking…';
    badgeStatusOllama.className = 'status-badge unchecked';

    try {
      const models = await invoke<AiModel[]>('fetch_ai_models', { provider: 'ollama' });
      if (models && models.length >= 0) {
        badgeStatusOllama.textContent = 'Online';
        badgeStatusOllama.className = 'status-badge online';
      }
    } catch {
      badgeStatusOllama.textContent = 'Offline';
      badgeStatusOllama.className = 'status-badge offline';
    }
  });

  // Refresh Models
  btnRefreshModels?.addEventListener('click', async () => {
    if (!btnRefreshModels) return;
    btnRefreshModels.disabled = true;
    const origHTML = btnRefreshModels.innerHTML;
    btnRefreshModels.innerHTML = `<span>Fetching…</span>`;

    try {
      const models = (await invoke<AiModel[]>('fetch_ai_models', { provider: 'all' })) || [];
      if (models.length > 0) {
        // Update model pickers with live list
        if (selAiDefaultModel) {
          const currentVal = selAiDefaultModel.value;
          selAiDefaultModel.innerHTML = '';
          for (const m of models) {
            const opt = document.createElement('option');
            opt.value = m.id;
            opt.textContent = `${m.name} (${m.provider})`;
            selAiDefaultModel.appendChild(opt);
          }
          if (models.some((m) => m.id === currentVal)) {
            selAiDefaultModel.value = currentVal;
          }
        }
        btnRefreshModels.innerHTML = `<span>Updated (${models.length}) ✓</span>`;
      } else {
        btnRefreshModels.innerHTML = `<span>No Models Found</span>`;
      }
    } catch (err) {
      console.error('Failed to refresh AI models:', err);
      btnRefreshModels.innerHTML = `<span>Error</span>`;
    }

    setTimeout(() => {
      if (btnRefreshModels) {
        btnRefreshModels.innerHTML = origHTML;
        btnRefreshModels.disabled = false;
      }
    }, 2500);
  });

  // Change Default AI Model
  selAiDefaultModel?.addEventListener('change', async () => {
    if (!selAiDefaultModel) return;
    await saveSetting('ai_default_model', selAiDefaultModel.value);
    await invoke('set_extension_preference', {
      extensionName: 'openai',
      key: 'model',
      value: selAiDefaultModel.value,
    });
    if (selExtPrefOpenaiModel) {
      selExtPrefOpenaiModel.value = selAiDefaultModel.value;
    }
  });

  // Extension Preferences: model
  selExtPrefOpenaiModel?.addEventListener('change', async () => {
    if (!selExtPrefOpenaiModel) return;
    await invoke('set_extension_preference', {
      extensionName: 'openai',
      key: 'model',
      value: selExtPrefOpenaiModel.value,
    });
    if (selAiDefaultModel) {
      selAiDefaultModel.value = selExtPrefOpenaiModel.value;
    }
  });

  // Extension Preferences: systemPrompt
  txtExtPrefOpenaiSystemPrompt?.addEventListener('change', async () => {
    if (!txtExtPrefOpenaiSystemPrompt) return;
    await invoke('set_extension_preference', {
      extensionName: 'openai',
      key: 'systemPrompt',
      value: txtExtPrefOpenaiSystemPrompt.value,
    });
  });

  // Export Preferences
  chkExportFirstLineTitle?.addEventListener('change', () =>
    saveSetting('exportGeneralUseFirstLineAsTitle', chkExportFirstLineTitle.checked)
  );
  chkExportOmitKeywords?.addEventListener('change', () =>
    saveSetting('exportGeneralOmitKeyword', chkExportOmitKeywords.checked)
  );
  chkExportPlainTextEnabled?.addEventListener('change', () =>
    saveSetting('exportPlainTextEnabled', chkExportPlainTextEnabled.checked)
  );
  chkExportMarkdownEnabled?.addEventListener('change', () =>
    saveSetting('exportMarkdownEnabled', chkExportMarkdownEnabled.checked)
  );
  chkExportObsidianEnabled?.addEventListener('change', () =>
    saveSetting('exportObsidianEnabled', chkExportObsidianEnabled.checked)
  );
  txtExportObsidianVault?.addEventListener('change', () =>
    saveSetting('exportObsidianVault', txtExportObsidianVault.value)
  );
  chkExportObsidianConvertMarkdown?.addEventListener('change', () =>
    saveSetting('exportObsidianConvertToMarkdown', chkExportObsidianConvertMarkdown.checked)
  );
  chkExportBearEnabled?.addEventListener('change', () =>
    saveSetting('exportBearEnabled', chkExportBearEnabled.checked)
  );
  chkExportBearConvertMarkdown?.addEventListener('change', () =>
    saveSetting('exportBearConvertToMarkdown', chkExportBearConvertMarkdown.checked)
  );
  chkExportAppleNotesEnabled?.addEventListener('change', () =>
    saveSetting('exportAppleNotesEnabled', chkExportAppleNotesEnabled.checked)
  );
  chkExportAppleNotesRichText?.addEventListener('change', () =>
    saveSetting('exportAppleNotesConvertToRichText', chkExportAppleNotesRichText.checked)
  );
  chkExportNotionEnabled?.addEventListener('change', () =>
    saveSetting('exportNotionEnabled', chkExportNotionEnabled.checked)
  );
  chkExportNotionSaveDatabase?.addEventListener('change', () =>
    saveSetting('exportNotionSaveInDatabase', chkExportNotionSaveDatabase.checked)
  );
  txtExportNotionDatabaseName?.addEventListener('change', () =>
    saveSetting('exportNotionDatabaseName', txtExportNotionDatabaseName.value)
  );
  chkExportCustomEnabled?.addEventListener('change', () =>
    saveSetting('exportCustomEnabled', chkExportCustomEnabled.checked)
  );
  txtExportCustomName?.addEventListener('change', () =>
    saveSetting('exportCustomName', txtExportCustomName.value)
  );
  txtExportCustomURLTemplate?.addEventListener('change', () =>
    saveSetting('exportCustomURLTemplate', txtExportCustomURLTemplate.value)
  );

  // Bulk ZIP Download
  btnSettingsExportZip?.addEventListener('click', async () => {
    if (!btnSettingsExportZip) return;
    const origText = btnSettingsExportZip.textContent || 'Download All Notes as ZIP';
    btnSettingsExportZip.disabled = true;
    btnSettingsExportZip.textContent = 'Generating ZIP archive...';

    try {
      const bytes = await invoke<number[]>('export_all_notes_zip', {
        options: {
          use_first_line_as_title: chkExportFirstLineTitle?.checked ?? true,
          omit_keywords: chkExportOmitKeywords?.checked ?? false,
          convert_to_markdown: chkExportMarkdownEnabled?.checked ?? true,
          convert_to_rich_text: false,
        },
      });

      if (bytes && bytes.length > 0) {
        const uint8 = new Uint8Array(bytes);
        const blob = new Blob([uint8], { type: 'application/zip' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        const now = new Date();
        const dateStr = now.toISOString().split('T')[0];
        a.download = `plainpad-notes-export-${dateStr}.zip`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        btnSettingsExportZip.textContent = 'ZIP Exported Successfully!';
      } else {
        btnSettingsExportZip.textContent = 'No Notes to Export';
      }
    } catch (err) {
      console.error('Failed to export all notes as ZIP:', err);
      btnSettingsExportZip.textContent = 'Export Failed';
    }

    setTimeout(() => {
      if (btnSettingsExportZip) {
        btnSettingsExportZip.textContent = origText;
        btnSettingsExportZip.disabled = false;
      }
    }, 2500);
  });
}

// ---------- Init ----------

setupTabs();
wireEvents();
loadSettings().catch(console.error);
