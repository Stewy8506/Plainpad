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

// Editor
const chkDefaultPlain = document.getElementById('setting-default-plain') as HTMLInputElement;
const chkGlobalPlain = document.getElementById('setting-global-plain') as HTMLInputElement;
const selTabWidth = document.getElementById('setting-tab-width') as HTMLSelectElement;
const chkAutoBrackets = document.getElementById('setting-auto-brackets') as HTMLInputElement;
const chkListCheckboxes = document.getElementById('setting-list-checkboxes') as HTMLInputElement;

// Keywords
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
  if (chkRestorePin && settings.restore_pin !== undefined) {
    chkRestorePin.checked = !!settings.restore_pin;
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
  if (chkListCheckboxes && settings.list_checkboxes !== undefined) {
    chkListCheckboxes.checked = !!settings.list_checkboxes;
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
  if (chkTimerSound && settings.timer_sound !== undefined) {
    chkTimerSound.checked = !!settings.timer_sound;
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
  chkRestorePin?.addEventListener('change', () => saveSetting('restore_pin', chkRestorePin.checked));
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

  // Editor
  chkDefaultPlain?.addEventListener('change', () => saveSetting('default_plain', chkDefaultPlain.checked));
  chkGlobalPlain?.addEventListener('change', () => saveSetting('global_plain', chkGlobalPlain.checked));
  selTabWidth?.addEventListener('change', () => saveSetting('tab_width', selTabWidth.value));
  chkAutoBrackets?.addEventListener('change', () => saveSetting('auto_brackets', chkAutoBrackets.checked));
  chkListCheckboxes?.addEventListener('change', () => saveSetting('list_checkboxes', chkListCheckboxes.checked));

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
  chkTimerSound?.addEventListener('change', () => saveSetting('timer_sound', chkTimerSound.checked));
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
}

// ---------- Init ----------

setupTabs();
wireEvents();
loadSettings().catch(console.error);
