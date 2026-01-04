async function checkStatus() {
  const wsIndicator = document.getElementById('wsIndicator');
  const wsStatus = document.getElementById('wsStatus');
  const tabIndicator = document.getElementById('tabIndicator');
  const tabStatus = document.getElementById('tabStatus');
  const apiIndicator = document.getElementById('apiIndicator');
  const apiStatus = document.getElementById('apiStatus');

  try {
    const testWs = new WebSocket('ws://127.0.0.1:9225');

    testWs.onopen = () => {
      wsIndicator.classList.add('connected');
      wsStatus.textContent = 'Connected';
      testWs.close();
    };

    testWs.onerror = () => {
      wsIndicator.classList.remove('connected');
      wsStatus.textContent = 'Not running';
    };
  } catch (e) {
    wsIndicator.classList.remove('connected');
    wsStatus.textContent = 'Error';
  }

  try {
    const tabs = await chrome.tabs.query({ url: 'http://localhost:8080/*' });

    if (tabs.length > 0) {
      tabIndicator.classList.add('connected');
      tabStatus.textContent = `Found (${tabs.length} tab${tabs.length > 1 ? 's' : ''})`;

      const tab = tabs[0];
      try {
        const result = await chrome.scripting.executeScript({
          target: { tabId: tab.id },
          world: 'MAIN',
          func: () => typeof window.__novywave_test_api !== 'undefined'
        });

        if (result && result[0] && result[0].result) {
          apiIndicator.classList.add('connected');
          apiStatus.textContent = 'Available';
        } else {
          apiIndicator.classList.remove('connected');
          apiStatus.textContent = 'Not exposed';
        }
      } catch (e) {
        apiIndicator.classList.remove('connected');
        apiStatus.textContent = 'Check error';
      }
    } else {
      tabIndicator.classList.remove('connected');
      tabStatus.textContent = 'Not found';
      apiIndicator.classList.remove('connected');
      apiStatus.textContent = 'N/A';
    }
  } catch (e) {
    tabIndicator.classList.remove('connected');
    tabStatus.textContent = 'Error';
  }
}

checkStatus();
setInterval(checkStatus, 3000);
