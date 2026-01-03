#!/bin/bash
# Deterministic test: count "Loading..." text in rendered HTML
# Uses Python + Selenium with HEADED Chrome to render WASM app
#
# Usage: ./scripts/test_loading_fix.sh [URL] [WAIT_SECONDS]
# Returns: 0 if PASS (0 Loading...), 1 if FAIL (Loading... remain)

set -e

URL="${1:-http://localhost:8080}"
WAIT_SECONDS="${2:-15}"

echo "=== Loading... Bug Fix Test ==="
echo "URL: $URL"
echo "Wait: ${WAIT_SECONDS}s for data to load"
echo ""

# Use Python + Selenium with HEADED Chrome
python3 << PYTHON_TEST
import sys
import time

try:
    from selenium import webdriver
    from selenium.webdriver.chrome.options import Options
    from selenium.webdriver.common.by import By
except ImportError:
    print("ERROR: selenium not found. Install with: pip install selenium")
    sys.exit(2)

url = "$URL"
wait_seconds = $WAIT_SECONDS

print("Launching Chrome (headed mode)...")
options = Options()
# HEADED mode - no --headless flag
options.add_argument('--no-sandbox')
options.add_argument('--disable-dev-shm-usage')
options.add_argument('--window-size=1920,1080')

try:
    driver = webdriver.Chrome(options=options)
except Exception as e:
    print(f"ERROR: Could not start Chrome: {e}")
    print("Make sure Chrome/Chromium and chromedriver are installed")
    sys.exit(2)

print(f"Navigating to {url}...")
try:
    driver.get(url)
except Exception as e:
    print(f"ERROR: Could not connect to {url}: {e}")
    print("Make sure the dev server is running (makers start)")
    driver.quit()
    sys.exit(2)

print(f"Waiting {wait_seconds}s for WASM app and data to load...")
time.sleep(wait_seconds)

# Get full HTML source
html = driver.page_source

# Count "Loading..." in raw HTML (most reliable)
html_count = html.count("Loading...")

# Also try to find elements containing "Loading..." text via XPath
try:
    loading_elements = driver.find_elements(By.XPATH, "//*[contains(text(), 'Loading...')]")
    xpath_count = len(loading_elements)
except:
    xpath_count = -1

# Try innerText as backup
try:
    body_text = driver.find_element(By.TAG_NAME, "body").text
    text_count = body_text.count("Loading...")
except:
    text_count = -1

driver.quit()

print(f"\n=== RESULT ===")
print(f"Found {html_count} instances of 'Loading...' in HTML source")
print(f"Found {xpath_count} elements with 'Loading...' via XPath")
print(f"Found {text_count} instances in body.innerText")

# Use HTML count as primary (most reliable for WASM-rendered content)
final_count = html_count

if final_count == 0:
    print("\n✓ PASS: No persistent Loading... states")
    sys.exit(0)
else:
    print(f"\n✗ FAIL: {final_count} Loading... instances remain in UI")
    sys.exit(1)
PYTHON_TEST
