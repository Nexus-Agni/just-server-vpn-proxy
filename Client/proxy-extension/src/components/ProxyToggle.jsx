import React, { useState, useEffect } from 'react';

const ProxyToggle = () => {
  const [isEnabled, setIsEnabled] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [serverStatus, setServerStatus] = useState('unknown');
  const [debugInfo, setDebugInfo] = useState('');

  // Load initial state
  useEffect(() => {
    setDebugInfo('Component loaded, checking initial state...');
    loadProxyState();
    checkServerStatus();
  }, []);

  const loadProxyState = async () => {
    try {
      if (typeof chrome !== 'undefined' && chrome.storage) {
        setDebugInfo('Loading proxy state from Chrome storage...');
        const result = await chrome.storage.sync.get('proxyEnabled');
        setIsEnabled(result.proxyEnabled || false);
        setDebugInfo(`Loaded proxy state: ${result.proxyEnabled || false}`);
      } else {
        // Fallback for development
        setDebugInfo('Chrome storage not available, using fallback');
        setIsEnabled(false);
      }
    } catch (error) {
      console.error('Error loading proxy state:', error);
      setDebugInfo(`Error loading state: ${error.message}`);
    } finally {
      setIsLoading(false);
    }
  };

  const checkServerStatus = async () => {
    try {
      if (typeof chrome !== 'undefined' && chrome.runtime) {
        // Use background script to check server status
        chrome.runtime.sendMessage(
          { action: 'checkServerStatus' },
          (response) => {
            if (response && response.online) {
              setServerStatus('online');
            } else {
              setServerStatus('offline');
            }
          }
        );
      } else {
        // Direct fetch for development
        const response = await fetch('http://localhost:8080/pqc-info', {
          method: 'GET',
          headers: {
            'Content-Type': 'application/json',
          },
        });
        
        if (response.ok) {
          setServerStatus('online');
        } else {
          setServerStatus('offline');
        }
      }
    } catch (error) {
      setServerStatus('offline');
    }
  };

  const toggleProxy = async () => {
    console.log('Toggle proxy clicked, current state:', isEnabled);
    setDebugInfo(`Toggling proxy from ${isEnabled} to ${!isEnabled}...`);
    setIsLoading(true);
    const newState = !isEnabled;
    
    try {
      if (typeof chrome !== 'undefined' && chrome.runtime) {
        console.log('Using Chrome runtime to toggle proxy to:', newState);
        setDebugInfo('Sending message to background script...');
        
        // Add timeout to prevent hanging
        const timeout = setTimeout(() => {
          console.error('Background script response timeout');
          setDebugInfo('Timeout waiting for background script response');
          setIsLoading(false);
        }, 5000);
        
        // Update via background script
        chrome.runtime.sendMessage({
          action: 'toggleProxy',
          enabled: newState
        }, (response) => {
          clearTimeout(timeout);
          console.log('Response from background script:', response);
          
          if (chrome.runtime.lastError) {
            console.error('Chrome runtime error:', chrome.runtime.lastError);
            setDebugInfo(`Chrome runtime error: ${chrome.runtime.lastError.message}`);
            setIsLoading(false);
            return;
          }
          
          if (response && response.success) {
            setIsEnabled(newState);
            setDebugInfo(`Proxy successfully ${newState ? 'enabled' : 'disabled'}`);
            console.log('Proxy toggled successfully to:', newState);
          } else {
            console.error('Failed to toggle proxy:', response);
            setDebugInfo(`Failed to toggle proxy: ${response ? response.error || 'Unknown error' : 'No response'}`);
          }
          setIsLoading(false);
        });
      } else {
        console.log('Chrome runtime not available, using fallback');
        setDebugInfo('Chrome runtime not available, using fallback');
        // Development fallback
        setIsEnabled(newState);
        setIsLoading(false);
      }
      
    } catch (error) {
      console.error('Error toggling proxy:', error);
      setDebugInfo(`Error: ${error.message}`);
      setIsEnabled(!newState);
      setIsLoading(false);
    }
  };

  const getStatusColor = () => {
    if (serverStatus === 'online') return 'text-green-600';
    if (serverStatus === 'offline') return 'text-red-600';
    return 'text-gray-400';
  };

  const getStatusText = () => {
    if (serverStatus === 'online') return 'Server Online';
    if (serverStatus === 'offline') return 'Server Offline';
    return 'Checking...';
  };

  return (
    <div className="w-80 h-auto min-h-96 bg-gradient-to-br from-blue-50 to-indigo-100 p-6 font-sans">
      {/* Header */}
      <div className="text-center mb-6">
        <div className="flex items-center justify-center mb-2">
          <div className="w-8 h-8 bg-gradient-to-r from-blue-600 to-purple-600 rounded-lg mr-3 flex items-center justify-center">
            <svg className="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
            </svg>
          </div>
          <h1 className="text-xl font-bold text-gray-800">PQC Proxy</h1>
        </div>
        <p className="text-sm text-gray-600">Post-Quantum Cryptography Protection</p>
      </div>

      {/* Server Status */}
      <div className="bg-white rounded-lg p-4 mb-6 shadow-sm">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium text-gray-700">Server Status</span>
          <div className="flex items-center">
            <div className={`w-2 h-2 rounded-full mr-2 ${
              serverStatus === 'online' ? 'bg-green-500' : 
              serverStatus === 'offline' ? 'bg-red-500' : 'bg-gray-400'
            }`}></div>
            <span className={`text-sm font-medium ${getStatusColor()}`}>
              {getStatusText()}
            </span>
          </div>
        </div>
        <div className="mt-2 text-xs text-gray-500">
          localhost:8080
        </div>
      </div>

      {/* Toggle Switch */}
      <div className="bg-white rounded-lg p-6 shadow-sm">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h3 className="text-lg font-semibold text-gray-800">Proxy Protection</h3>
            <p className="text-sm text-gray-600">
              {isEnabled ? 'All traffic routed through PQC proxy' : 'Direct connection active'}
            </p>
          </div>
        </div>
        
        <div className="flex items-center justify-center mb-4">
          <label className="relative inline-flex items-center cursor-pointer">
            <input
              type="checkbox"
              className="sr-only peer"
              checked={isEnabled}
              onChange={toggleProxy}
              disabled={isLoading || serverStatus === 'offline'}
            />
            <div className={`relative w-16 h-8 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 rounded-full peer peer-checked:after:translate-x-8 peer-checked:after:border-white after:content-[''] after:absolute after:top-0.5 after:left-[4px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-7 after:w-7 after:transition-all peer-checked:bg-gradient-to-r peer-checked:from-blue-600 peer-checked:to-purple-600 ${
              isLoading || serverStatus === 'offline' ? 'opacity-50 cursor-not-allowed' : ''
            }`}></div>
          </label>
        </div>

        {/* Backup Toggle Button */}
        <div className="flex items-center justify-center">
          <button
            onClick={toggleProxy}
            disabled={isLoading || serverStatus === 'offline'}
            className={`px-6 py-3 rounded-lg font-semibold text-sm transition-all duration-200 ${
              isEnabled 
                ? 'bg-gradient-to-r from-red-500 to-red-600 hover:from-red-600 hover:to-red-700 text-white' 
                : 'bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700 text-white'
            } ${
              isLoading || serverStatus === 'offline' 
                ? 'opacity-50 cursor-not-allowed' 
                : 'hover:shadow-lg transform hover:scale-105'
            }`}
          >
            {isLoading ? 'Updating...' : isEnabled ? 'Disable Proxy' : 'Enable Proxy'}
          </button>
        </div>

        {isLoading && (
          <div className="flex items-center justify-center mt-4">
            <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-blue-600"></div>
            <span className="ml-2 text-sm text-gray-600">Updating...</span>
          </div>
        )}
      </div>

      {/* Status Info */}
      <div className="mt-6 bg-white rounded-lg p-4 shadow-sm">
        <h4 className="text-sm font-semibold text-gray-800 mb-2">Current Status</h4>
        <div className="space-y-2 text-xs">
          <div className="flex justify-between">
            <span className="text-gray-600">Connection:</span>
            <span className={`font-medium ${isEnabled ? 'text-blue-600' : 'text-gray-600'}`}>
              {isEnabled ? 'Proxied' : 'Direct'}
            </span>
          </div>
          <div className="flex justify-between">
            <span className="text-gray-600">Encryption:</span>
            <span className={`font-medium ${isEnabled ? 'text-green-600' : 'text-gray-600'}`}>
              {isEnabled ? 'Quantum-Safe' : 'Standard'}
            </span>
          </div>
          <div className="flex justify-between">
            <span className="text-gray-600">Algorithm:</span>
            <span className="text-gray-600">
              {isEnabled ? 'Kyber-768' : 'None'}
            </span>
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="mt-4 text-center">
        <button
          onClick={checkServerStatus}
          className="text-xs text-blue-600 hover:text-blue-800 font-medium"
        >
          Refresh Server Status
        </button>
      </div>

      {/* Debug Info */}
      {debugInfo && (
        <div className="mt-4 bg-gray-100 rounded-lg p-3">
          <h5 className="text-xs font-semibold text-gray-700 mb-1">Debug Info</h5>
          <p className="text-xs text-gray-600">{debugInfo}</p>
          <p className="text-xs text-gray-500 mt-1">
            Chrome API: {typeof chrome !== 'undefined' ? 'Available' : 'Not Available'}
          </p>
        </div>
      )}
    </div>
  );
};

export default ProxyToggle;
