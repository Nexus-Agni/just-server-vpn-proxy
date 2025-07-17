// Background service worker for Chrome extension
const PROXY_SERVER = 'http://localhost:8080';

chrome.runtime.onInstalled.addListener(() => {
  console.log('PQC Proxy Extension installed');
  
  // Initialize storage with default values
  chrome.storage.sync.set({
    proxyEnabled: false
  });
  
  // Set initial badge
  chrome.action.setBadgeText({ text: "" });
});

// Listen for storage changes to update proxy rules
chrome.storage.onChanged.addListener((changes, namespace) => {
  if (namespace === 'sync' && changes.proxyEnabled) {
    const isEnabled = changes.proxyEnabled.newValue;
    updateProxyRules(isEnabled);
  }
});

// Function to update declarative net request rules
async function updateProxyRules(enabled) {
  console.log(`Updating proxy rules: ${enabled ? 'enabling' : 'disabling'} proxy`);
  try {
    if (enabled) {
      // Enable proxy rules
      await chrome.declarativeNetRequest.updateEnabledRulesets({
        enableRulesetIds: ['proxy_rules']
      });
      
      // Update extension badge
      chrome.action.setBadgeText({ text: "ON" });
      chrome.action.setBadgeBackgroundColor({ color: "#10B981" });
      
      console.log('Proxy enabled - redirecting traffic through PQC proxy');
      
    } else {
      // Disable proxy rules
      await chrome.declarativeNetRequest.updateEnabledRulesets({
        disableRulesetIds: ['proxy_rules']
      });
      
      // Clear badge
      chrome.action.setBadgeText({ text: "" });
      
      console.log('Proxy disabled - using direct connection');
    }
    
    console.log(`Proxy rules updated successfully: ${enabled ? 'enabled' : 'disabled'}`);
    return true;
    
  } catch (error) {
    console.error('Error updating proxy rules:', error);
    // Revert badge state on error
    chrome.action.setBadgeText({ text: "ERR" });
    chrome.action.setBadgeBackgroundColor({ color: "#EF4444" });
    throw error;
  }
}

// Handle extension startup
chrome.runtime.onStartup.addListener(async () => {
  const result = await chrome.storage.sync.get('proxyEnabled');
  updateProxyRules(result.proxyEnabled || false);
});

// Message handling for popup communication
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === 'getProxyStatus') {
    chrome.storage.sync.get('proxyEnabled', (result) => {
      sendResponse({ enabled: result.proxyEnabled || false });
    });
    return true; // Keep the message channel open for async response
  }
  
  if (request.action === 'toggleProxy') {
    chrome.storage.sync.set({ 
      proxyEnabled: request.enabled 
    }, async () => {
      try {
        await updateProxyRules(request.enabled);
        sendResponse({ success: true, enabled: request.enabled });
      } catch (error) {
        console.error('Error updating proxy rules:', error);
        sendResponse({ success: false, error: error.message });
      }
    });
    return true;
  }
  
  if (request.action === 'checkServerStatus') {
    // Check if proxy server is reachable
    fetch(`${PROXY_SERVER}/pqc-info`)
      .then(response => {
        sendResponse({ 
          online: response.ok,
          status: response.status 
        });
      })
      .catch(error => {
        sendResponse({ 
          online: false,
          error: error.message 
        });
      });
    return true;
  }
});
