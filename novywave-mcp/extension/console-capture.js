(function() {
  if (typeof window.__novywaveCapturedConsole !== 'undefined') return;

  window.__novywaveCapturedConsole = [];
  const maxMessages = 500;

  const originalConsole = {
    log: console.log.bind(console),
    warn: console.warn.bind(console),
    error: console.error.bind(console),
    info: console.info.bind(console),
    debug: console.debug.bind(console)
  };

  function captureConsole(level, ...args) {
    const text = args.map(arg =>
      typeof arg === 'object' ? JSON.stringify(arg) : String(arg)
    ).join(' ');

    window.__novywaveCapturedConsole.push({
      level,
      text,
      timestamp: Date.now()
    });

    if (window.__novywaveCapturedConsole.length > maxMessages) {
      window.__novywaveCapturedConsole.shift();
    }

    originalConsole[level](...args);
  }

  console.log = (...args) => captureConsole('log', ...args);
  console.warn = (...args) => captureConsole('warn', ...args);
  console.error = (...args) => captureConsole('error', ...args);
  console.info = (...args) => captureConsole('info', ...args);
  console.debug = (...args) => captureConsole('debug', ...args);
})();
