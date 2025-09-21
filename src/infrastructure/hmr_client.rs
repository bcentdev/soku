/// HMR Client-side JavaScript runtime
/// This generates the JavaScript code that gets injected into the bundle
/// to enable hot reloading in the browser

pub fn generate_hmr_client_code(port: u16) -> String {
    format!(r#"
// Ultra Bundler HMR Client Runtime
(function() {{
    'use strict';

    const HMR_PORT = {port};
    const HMR_URL = `ws://localhost:${{HMR_PORT}}`;

    class UltraHMRClient {{
        constructor() {{
            this.ws = null;
            this.reconnectAttempts = 0;
            this.maxReconnectAttempts = 10;
            this.reconnectDelay = 1000;
            this.moduleCache = new Map();
            this.cssLinks = new Map();
            this.isConnected = false;

            this.connect();
            this.setupErrorHandling();
        }}

        connect() {{
            try {{
                this.ws = new WebSocket(HMR_URL);
                this.setupEventHandlers();
            }} catch (error) {{
                console.warn('[Ultra HMR] Connection failed:', error);
                this.scheduleReconnect();
            }}
        }}

        setupEventHandlers() {{
            this.ws.onopen = () => {{
                this.isConnected = true;
                this.reconnectAttempts = 0;
                this.showNotification('🔥 Ultra HMR Connected', 'success');
                console.log('[Ultra HMR] Connected to development server');
            }};

            this.ws.onmessage = (event) => {{
                try {{
                    const update = JSON.parse(event.data);
                    this.handleUpdate(update);
                }} catch (error) {{
                    console.warn('[Ultra HMR] Invalid update message:', error);
                }}
            }};

            this.ws.onclose = () => {{
                this.isConnected = false;
                console.log('[Ultra HMR] Connection closed');
                this.scheduleReconnect();
            }};

            this.ws.onerror = (error) => {{
                console.warn('[Ultra HMR] WebSocket error:', error);
            }};
        }}

        handleUpdate(update) {{
            console.log('[Ultra HMR] Received update:', update);

            switch (update.kind) {{
                case 'FullReload':
                    this.performFullReload();
                    break;

                case 'ModuleUpdated':
                    this.updateModule(update);
                    break;

                case 'CssUpdated':
                    this.updateCss(update);
                    break;

                case 'FileAdded':
                case 'FileRemoved':
                    // For now, trigger full reload for file additions/removals
                    this.performFullReload();
                    break;

                default:
                    console.log('[Ultra HMR] Unknown update kind:', update.kind);
            }}
        }}

        updateModule(update) {{
            const modulePath = update.path;
            const newContent = update.content;

            if (!newContent) {{
                console.warn('[Ultra HMR] No content for module update:', modulePath);
                return;
            }}

            try {{
                // Attempt hot module replacement
                if (this.canHotReplace(modulePath)) {{
                    this.hotReplaceModule(modulePath, newContent);
                    this.showNotification(`📦 Updated: ${{modulePath}}`, 'info');
                }} else {{
                    // Fallback to full reload
                    this.performFullReload();
                }}
            }} catch (error) {{
                console.warn('[Ultra HMR] Module update failed:', error);
                this.performFullReload();
            }}
        }}

        updateCss(update) {{
            const cssPath = update.path;
            const newContent = update.content;

            if (!newContent) {{
                console.warn('[Ultra HMR] No content for CSS update:', cssPath);
                return;
            }}

            try {{
                this.hotReplaceCss(cssPath, newContent);
                this.showNotification(`🎨 CSS Updated: ${{cssPath}}`, 'info');
            }} catch (error) {{
                console.warn('[Ultra HMR] CSS update failed:', error);
                this.performFullReload();
            }}
        }}

        canHotReplace(modulePath) {{
            // Simple heuristic: can hot replace if it's not the main entry
            // and doesn't contain certain keywords that require full reload
            const content = this.moduleCache.get(modulePath) || '';
            const hasComplexUpdates = /(?:class\s+\w+|function\s+\w+|const\s+\w+\s*=\s*\w+|export\s+default)/.test(content);

            return !hasComplexUpdates;
        }}

        hotReplaceModule(modulePath, newContent) {{
            // Store the new content
            this.moduleCache.set(modulePath, newContent);

            // Create a new script element with the updated content
            const script = document.createElement('script');
            script.type = 'module';
            script.textContent = newContent;

            // Remove old script if exists
            const oldScript = document.querySelector(`script[data-ultra-module="${{modulePath}}"]`);
            if (oldScript) {{
                oldScript.remove();
            }}

            // Add new script
            script.setAttribute('data-ultra-module', modulePath);
            document.head.appendChild(script);

            // Trigger custom event for application to handle
            window.dispatchEvent(new CustomEvent('ultra-hmr-module-updated', {{
                detail: {{ modulePath, content: newContent }}
            }}));
        }}

        hotReplaceCss(cssPath, newContent) {{
            // Find existing CSS link or style tag
            let cssElement = this.cssLinks.get(cssPath);

            if (!cssElement) {{
                // Look for existing link tags
                cssElement = document.querySelector(`link[href*="${{cssPath}}"]`);
                if (!cssElement) {{
                    // Look for existing style tags
                    cssElement = document.querySelector(`style[data-ultra-css="${{cssPath}}"]`);
                }}
            }}

            if (cssElement && cssElement.tagName === 'LINK') {{
                // For link tags, create a new one with cache busting
                const newLink = document.createElement('link');
                newLink.rel = 'stylesheet';
                newLink.href = cssElement.href + '?t=' + Date.now();
                newLink.onload = () => cssElement.remove();
                cssElement.parentNode.insertBefore(newLink, cssElement.nextSibling);
                this.cssLinks.set(cssPath, newLink);
            }} else {{
                // For inline styles or if no existing element, create new style tag
                if (cssElement) {{
                    cssElement.remove();
                }}

                const style = document.createElement('style');
                style.setAttribute('data-ultra-css', cssPath);
                style.textContent = newContent;
                document.head.appendChild(style);
                this.cssLinks.set(cssPath, style);
            }}

            // Trigger custom event
            window.dispatchEvent(new CustomEvent('ultra-hmr-css-updated', {{
                detail: {{ cssPath, content: newContent }}
            }}));
        }}

        performFullReload() {{
            this.showNotification('🔄 Reloading page...', 'warning');
            console.log('[Ultra HMR] Performing full page reload');

            // Small delay to show notification
            setTimeout(() => {{
                window.location.reload();
            }}, 300);
        }}

        scheduleReconnect() {{
            if (this.reconnectAttempts >= this.maxReconnectAttempts) {{
                console.warn('[Ultra HMR] Max reconnection attempts reached');
                this.showNotification('❌ HMR Disconnected', 'error');
                return;
            }}

            this.reconnectAttempts++;
            const delay = this.reconnectDelay * this.reconnectAttempts;

            console.log(`[Ultra HMR] Reconnecting in ${{delay}}ms (attempt ${{this.reconnectAttempts}})`);

            setTimeout(() => {{
                this.connect();
            }}, delay);
        }}

        showNotification(message, type = 'info') {{
            // Create notification element
            const notification = document.createElement('div');
            notification.style.cssText = `
                position: fixed;
                top: 20px;
                right: 20px;
                padding: 12px 16px;
                border-radius: 6px;
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', monospace;
                font-size: 14px;
                font-weight: 500;
                color: white;
                z-index: 10000;
                max-width: 300px;
                box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
                transition: all 0.3s ease;
                opacity: 0;
                transform: translateX(100%);
            `;

            // Set background color based on type
            const colors = {{
                success: '#10b981',
                info: '#3b82f6',
                warning: '#f59e0b',
                error: '#ef4444'
            }};
            notification.style.backgroundColor = colors[type] || colors.info;

            notification.textContent = message;
            document.body.appendChild(notification);

            // Animate in
            requestAnimationFrame(() => {{
                notification.style.opacity = '1';
                notification.style.transform = 'translateX(0)';
            }});

            // Auto remove
            setTimeout(() => {{
                notification.style.opacity = '0';
                notification.style.transform = 'translateX(100%)';
                setTimeout(() => {{
                    if (notification.parentNode) {{
                        notification.parentNode.removeChild(notification);
                    }}
                }}, 300);
            }}, 3000);
        }}

        setupErrorHandling() {{
            // Handle unhandled promise rejections
            window.addEventListener('unhandledrejection', (event) => {{
                console.warn('[Ultra HMR] Unhandled promise rejection:', event.reason);
            }});

            // Handle JavaScript errors
            window.addEventListener('error', (event) => {{
                console.warn('[Ultra HMR] JavaScript error:', event.error);
            }});
        }}

        // Public API
        isConnected() {{
            return this.isConnected;
        }}

        getStats() {{
            return {{
                connected: this.isConnected,
                reconnectAttempts: this.reconnectAttempts,
                cachedModules: this.moduleCache.size,
                cachedCss: this.cssLinks.size
            }};
        }}
    }}

    // Initialize HMR client
    if (typeof window !== 'undefined') {{
        window.__ULTRA_HMR__ = new UltraHMRClient();

        // Expose some utilities for debugging
        window.__ULTRA_HMR_DEBUG__ = {{
            reload: () => window.__ULTRA_HMR__.performFullReload(),
            stats: () => window.__ULTRA_HMR__.getStats(),
            isConnected: () => window.__ULTRA_HMR__.isConnected()
        }};

        console.log('🔥 Ultra HMR Client initialized');
    }}
}})();
"#, port = port)
}