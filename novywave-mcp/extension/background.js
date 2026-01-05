const WS_URL = 'ws://127.0.0.1:9225';
let ws = null;
let reconnectTimer = null;
let reconnectAttempts = 0;
const MAX_RECONNECT_DELAY = 30000;

let debuggerAttached = new Map();
let cdpConsoleMessages = new Map();
let cachedNovyWaveTabId = null;

async function attachDebugger(tabId) {
  if (debuggerAttached.get(tabId)) return;

  try {
    await chrome.debugger.attach({ tabId }, '1.3');
    debuggerAttached.set(tabId, true);
    console.log(`[NovyWave] CDP: Debugger attached to tab ${tabId}`);

    await chrome.debugger.sendCommand({ tabId }, 'DOM.enable');
    await chrome.debugger.sendCommand({ tabId }, 'Runtime.enable');
    await chrome.debugger.sendCommand({ tabId }, 'Page.enable');
  } catch (e) {
    if (e.message && e.message.includes('Another debugger is already attached')) {
      console.log('[NovyWave] CDP: Another debugger attached, trying to reuse...');
      debuggerAttached.set(tabId, true);
      try {
        await chrome.debugger.sendCommand({ tabId }, 'DOM.enable');
        await chrome.debugger.sendCommand({ tabId }, 'Runtime.enable');
        await chrome.debugger.sendCommand({ tabId }, 'Page.enable');
        return;
      } catch (e2) {
        debuggerAttached.delete(tabId);
        throw new Error('CDP debugger conflict. Close Chrome DevTools or use novywave_detach.');
      }
    }
    throw e;
  }
}

chrome.debugger.onEvent.addListener((source, method, params) => {
  if (method === 'Runtime.consoleAPICalled') {
    const messages = cdpConsoleMessages.get(source.tabId) || [];
    messages.push({
      level: params.type,
      text: params.args.map(arg => arg.value || arg.description || '').join(' '),
      timestamp: Date.now()
    });
    if (messages.length > 2000) messages.shift();
    cdpConsoleMessages.set(source.tabId, messages);
  }
  if (method === 'Runtime.exceptionThrown') {
    const messages = cdpConsoleMessages.get(source.tabId) || [];
    const exception = params.exceptionDetails;
    messages.push({
      level: 'error',
      text: `[EXCEPTION] ${exception.exception?.description || exception.text || 'Unknown'}`,
      timestamp: Date.now()
    });
    if (messages.length > 2000) messages.shift();
    cdpConsoleMessages.set(source.tabId, messages);
  }
});

chrome.debugger.onDetach.addListener((source, reason) => {
  console.log(`[NovyWave] CDP: Debugger detached from tab ${source.tabId}, reason: ${reason}`);
  debuggerAttached.delete(source.tabId);
  cdpConsoleMessages.delete(source.tabId);
});

chrome.tabs.onUpdated.addListener((tabId, changeInfo) => {
  if (changeInfo.status === 'loading' && debuggerAttached.has(tabId)) {
    console.log(`[NovyWave] CDP: Tab ${tabId} navigating, clearing debugger state`);
    debuggerAttached.delete(tabId);
    cdpConsoleMessages.delete(tabId);
  }
});

chrome.tabs.onRemoved.addListener((tabId) => {
  if (tabId === cachedNovyWaveTabId) {
    cachedNovyWaveTabId = null;
  }
});

async function cdpEvaluate(tabId, expression) {
  await attachDebugger(tabId);
  const result = await chrome.debugger.sendCommand({ tabId }, 'Runtime.evaluate', {
    expression,
    returnByValue: true
  });
  return result?.result?.value;
}

async function cdpScreenshot(tabId) {
  await attachDebugger(tabId);
  const result = await chrome.debugger.sendCommand({ tabId }, 'Page.captureScreenshot', {
    format: 'png'
  });
  return result.data;
}

async function cdpScreenshotElement(tabId, selector) {
  await attachDebugger(tabId);
  const box = await cdpEvaluate(tabId, `
    (function() {
      const el = document.querySelector('${selector}');
      if (!el) return null;
      const rect = el.getBoundingClientRect();
      return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
    })()
  `);
  if (!box) throw new Error(`Element not found: ${selector}`);

  const result = await chrome.debugger.sendCommand({ tabId }, 'Page.captureScreenshot', {
    format: 'png',
    clip: { x: box.x, y: box.y, width: box.width, height: box.height, scale: 1 }
  });
  return result.data;
}

function cdpGetConsole(tabId) {
  return cdpConsoleMessages.get(tabId) || [];
}

async function cdpPressKey(tabId, key, shift = false) {
  await attachDebugger(tabId);

  const keyCode = key.charCodeAt(0);
  const modifiers = shift ? 8 : 0;

  await chrome.debugger.sendCommand({ tabId }, 'Input.dispatchKeyEvent', {
    type: 'keyDown',
    key: key,
    code: `Key${key.toUpperCase()}`,
    text: key,
    unmodifiedText: key,
    windowsVirtualKeyCode: keyCode,
    nativeVirtualKeyCode: keyCode,
    modifiers
  });

  await chrome.debugger.sendCommand({ tabId }, 'Input.dispatchKeyEvent', {
    type: 'keyUp',
    key: key,
    code: `Key${key.toUpperCase()}`,
    windowsVirtualKeyCode: keyCode,
    nativeVirtualKeyCode: keyCode,
    modifiers
  });

  await cdpEvaluate(tabId, `
    (function() {
      document.dispatchEvent(new KeyboardEvent('keydown', {
        key: '${key}',
        code: 'Key${key.toUpperCase()}',
        keyCode: ${keyCode},
        which: ${keyCode},
        shiftKey: ${shift},
        bubbles: true,
        cancelable: true
      }));
      document.dispatchEvent(new KeyboardEvent('keyup', {
        key: '${key}',
        code: 'Key${key.toUpperCase()}',
        keyCode: ${keyCode},
        which: ${keyCode},
        shiftKey: ${shift},
        bubbles: true,
        cancelable: true
      }));
    })()
  `);
}

async function cdpTypeText(tabId, text) {
  await attachDebugger(tabId);
  await chrome.debugger.sendCommand({ tabId }, 'Input.insertText', { text });

  await cdpEvaluate(tabId, `
    (function() {
      const el = document.activeElement;
      if (el) {
        el.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: '${text}' }));
        el.dispatchEvent(new Event('change', { bubbles: true }));
      }
    })()
  `);
}

async function cdpClickText(tabId, text, exact) {
  await attachDebugger(tabId);

  const result = await cdpEvaluate(tabId, `
    (function() {
      const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, null, false);
      let node;
      while (node = walker.nextNode()) {
        const nodeText = node.textContent.trim();
        const matches = ${exact} ? nodeText === '${text}' : nodeText.includes('${text}');
        if (matches && nodeText.length > 0) {
          const el = node.parentElement;
          if (el) {
            const rect = el.getBoundingClientRect();
            if (rect.width > 0 && rect.height > 0) {
              el.click();
              return { success: true, tag: el.tagName, x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
            }
          }
        }
      }
      return { success: false, error: 'Element not found: ${text}' };
    })()
  `);

  return result;
}

function connect() {
  if (ws && (ws.readyState === WebSocket.CONNECTING || ws.readyState === WebSocket.OPEN)) {
    return;
  }

  console.log('[NovyWave] Connecting to WebSocket server...');

  try {
    ws = new WebSocket(WS_URL);
  } catch (e) {
    console.error('[NovyWave] WebSocket constructor error:', e);
    scheduleReconnect();
    return;
  }

  ws.onopen = () => {
    console.log('[NovyWave] Connected to WebSocket server');
    reconnectAttempts = 0;
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    safeSend({ clientType: 'extension' });
  };

  ws.onclose = () => {
    console.log('[NovyWave] WebSocket connection closed');
    ws = null;
    scheduleReconnect();
  };

  ws.onerror = (error) => {
    console.error('[NovyWave] WebSocket error:', error);
  };

  ws.onmessage = async (event) => {
    try {
      const request = JSON.parse(event.data);
      console.log('[NovyWave] Received request:', request);

      const response = await handleCommand(request.id, request.command);

      if (response !== null) {
        safeSend({ id: request.id, response });
        console.log('[NovyWave] Sent response:', response);
      }
    } catch (e) {
      console.error('[NovyWave] Error handling message:', e);
    }
  };
}

function safeSend(data) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(data));
  }
}

function scheduleReconnect() {
  if (reconnectTimer) return;

  const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), MAX_RECONNECT_DELAY);
  reconnectAttempts++;

  console.log(`[NovyWave] Scheduling reconnect in ${delay}ms (attempt ${reconnectAttempts})`);

  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    connect();
  }, delay);
}

async function handleCommand(id, command) {
  const type = command.type;

  try {
    let tab = null;

    if (cachedNovyWaveTabId !== null) {
      try {
        tab = await chrome.tabs.get(cachedNovyWaveTabId);
        if (!tab.url || !tab.url.startsWith('http://localhost:8080')) {
          cachedNovyWaveTabId = null;
          tab = null;
        }
      } catch (e) {
        cachedNovyWaveTabId = null;
        tab = null;
      }
    }

    if (tab === null) {
      const tabs = await chrome.tabs.query({ url: 'http://localhost:8080/*' });
      if (tabs.length === 0) {
        return { type: 'error', message: 'No NovyWave tab found (localhost:8080)' };
      }
      tab = tabs.find(t => t.active) || tabs[0];
      cachedNovyWaveTabId = tab.id;
    }

    switch (type) {
      case 'ping':
        return { type: 'pong' };

      case 'getStatus':
        const appReady = await cdpEvaluate(tab.id, `typeof window.__novywave_test_api !== 'undefined'`);
        return {
          type: 'status',
          connected: true,
          pageUrl: tab.url,
          appReady: !!appReady
        };

      case 'screenshot':
        try {
          const base64 = await cdpScreenshot(tab.id);
          return { type: 'screenshot', base64 };
        } catch (e) {
          return { type: 'error', message: `Screenshot failed: ${e.message}` };
        }

      case 'screenshotCanvas':
        try {
          const base64 = await cdpScreenshotElement(tab.id, 'canvas');
          return { type: 'screenshot', base64 };
        } catch (e) {
          return { type: 'error', message: `Canvas screenshot failed: ${e.message}` };
        }

      case 'screenshotElement':
        try {
          const base64 = await cdpScreenshotElement(tab.id, command.selector);
          return { type: 'screenshot', base64 };
        } catch (e) {
          return { type: 'error', message: `Element screenshot failed: ${e.message}` };
        }

      case 'getConsole':
        return { type: 'console', messages: cdpGetConsole(tab.id) };

      case 'refresh':
        await chrome.tabs.reload(tab.id);
        return { type: 'success', data: null };

      case 'detach':
        if (debuggerAttached.get(tab.id)) {
          await chrome.debugger.detach({ tabId: tab.id });
          debuggerAttached.delete(tab.id);
        }
        return { type: 'success', data: 'Debugger detached' };

      case 'reload':
        console.log('[NovyWave] Reloading extension...');
        safeSend({ id, response: { type: 'success', data: null } });
        setTimeout(() => {
          chrome.runtime.reload();
        }, 100);
        return null;

      case 'pressKey':
        await cdpPressKey(tab.id, command.key, command.shift || false);
        return { type: 'success', data: { key: command.key, shift: command.shift } };

      case 'typeText':
        await cdpTypeText(tab.id, command.text);
        return { type: 'success', data: { text: command.text } };

      case 'click':
        try {
          await cdpEvaluate(tab.id, `document.querySelector('${command.selector}')?.click()`);
          return { type: 'success', data: { selector: command.selector } };
        } catch (e) {
          return { type: 'error', message: e.message };
        }

      case 'clickAt':
        await cdpEvaluate(tab.id, `
          (function() {
            const el = document.elementFromPoint(${command.x}, ${command.y});
            if (el) el.click();
          })()
        `);
        return { type: 'success', data: { x: command.x, y: command.y } };

      case 'clickText':
        const clickResult = await cdpClickText(tab.id, command.text, command.exact);
        if (clickResult.success) {
          return { type: 'success', data: clickResult };
        } else {
          return { type: 'error', message: clickResult.error };
        }

      case 'findText':
        try {
          const searchText = command.text;
          const exactMatch = command.exact;
          const findResult = await cdpEvaluate(tab.id, `
            (function() {
              const matches = [];
              const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, null, false);
              let node;
              while (node = walker.nextNode()) {
                const nodeText = node.textContent.trim();
                if (nodeText.length === 0) continue;
                const found = ${exactMatch} ? nodeText === '${searchText}' : nodeText.includes('${searchText}');
                if (found) {
                  matches.push(nodeText.substring(0, 100));
                }
              }
              return { found: matches.length > 0, count: matches.length, matches: matches.slice(0, 10) };
            })()
          `);
          return { type: 'textMatches', ...findResult };
        } catch (e) {
          return { type: 'error', message: e.message };
        }

      case 'getPageText':
        try {
          const pageText = await cdpEvaluate(tab.id, `document.body.innerText`);
          return { type: 'pageText', text: pageText || '' };
        } catch (e) {
          return { type: 'error', message: e.message };
        }

      case 'getTimelineState':
        try {
          const state = await cdpEvaluate(tab.id, `window.__novywave_test_api?.getTimelineState()`);
          if (state) {
            return { type: 'timelineState', ...state };
          }
          return { type: 'jsResult', result: state };
        } catch (e) {
          return { type: 'error', message: e.message };
        }

      case 'getCursorValues':
        try {
          const values = await cdpEvaluate(tab.id, `window.__novywave_test_api?.getCursorValues()`);
          return { type: 'cursorValues', values: values || {} };
        } catch (e) {
          return { type: 'error', message: e.message };
        }

      case 'getSelectedVariables':
        try {
          const variables = await cdpEvaluate(tab.id, `window.__novywave_test_api?.getSelectedVariables()`);
          return { type: 'selectedVariables', variables: variables || [] };
        } catch (e) {
          return { type: 'error', message: e.message };
        }

      case 'getLoadedFiles':
        try {
          const files = await cdpEvaluate(tab.id, `window.__novywave_test_api?.getLoadedFiles()`);
          return { type: 'loadedFiles', files: files || [] };
        } catch (e) {
          return { type: 'error', message: e.message };
        }

      case 'evaluateJs':
        try {
          const result = await cdpEvaluate(tab.id, command.script);
          return { type: 'jsResult', result };
        } catch (e) {
          return { type: 'error', message: e.message };
        }

      case 'selectWorkspace':
        try {
          // Use direct evaluation to trigger workspace selection via the app's existing mechanisms
          // This sends a click to the workspace selector which will open a dialog,
          // or we can use the test API if available
          const result = await cdpEvaluate(tab.id, `
            (function() {
              // Try the test API first
              if (window.__novywave_test_api?.selectWorkspace) {
                return window.__novywave_test_api.selectWorkspace('${command.path.replace(/'/g, "\\'")}');
              }
              // Fallback: Try to access the Rust function directly
              if (typeof window.__novywave_select_workspace === 'function') {
                window.__novywave_select_workspace('${command.path.replace(/'/g, "\\'")}');
                return true;
              }
              console.error('[NovyWave] selectWorkspace: No method available');
              return false;
            })()
          `);
          return { type: 'success', data: { path: command.path, result } };
        } catch (e) {
          return { type: 'error', message: e.message };
        }

      default:
        return { type: 'error', message: `Unknown command: ${type}` };
    }
  } catch (e) {
    console.error('[NovyWave] Command error:', e);
    return { type: 'error', message: e.message };
  }
}

chrome.runtime.onInstalled.addListener(() => {
  console.log('[NovyWave] Extension installed/updated');
  connect();
});

chrome.runtime.onStartup.addListener(() => {
  console.log('[NovyWave] Browser started');
  connect();
});

chrome.alarms.create('keepalive', { periodInMinutes: 0.5 });
chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === 'keepalive') {
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      connect();
    }
  }
});

connect();
