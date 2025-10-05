/// HMR Client-side JavaScript runtime
/// This generates the JavaScript code that gets injected into the bundle
/// to enable hot reloading in the browser
pub fn generate_hmr_client_code(port: u16) -> String {
    format!(
        r#"
// Soku Bundler HMR Client Runtime
(function() {{
    'use strict';

    const HMR_PORT = {port};
    const HMR_URL = `ws://localhost:${{HMR_PORT}}`;

    class SokuHMRClient {{
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
                console.warn('[Soku HMR] Connection failed:', error);
                this.scheduleReconnect();
            }}
        }}

        setupEventHandlers() {{
            this.ws.onopen = () => {{
                this.isConnected = true;
                this.reconnectAttempts = 0;
                this.showNotification('🔥 Soku HMR Connected', 'success');
                console.log('[Soku HMR] Connected to development server');
            }};

            this.ws.onmessage = (event) => {{
                try {{
                    const update = JSON.parse(event.data);
                    this.handleUpdate(update);
                }} catch (error) {{
                    console.warn('[Soku HMR] Invalid update message:', error);
                }}
            }};

            this.ws.onclose = () => {{
                this.isConnected = false;
                console.log('[Soku HMR] Connection closed');
                this.scheduleReconnect();
            }};

            this.ws.onerror = (error) => {{
                console.warn('[Soku HMR] WebSocket error:', error);
            }};
        }}

        handleUpdate(update) {{
            console.log('[Soku HMR] Received update:', update);

            switch (update.kind) {{
                case 'FullReload':
                    this.hideErrorOverlay();
                    this.performFullReload();
                    break;

                case 'ModuleUpdated':
                    this.hideErrorOverlay();
                    this.updateModule(update);
                    break;

                case 'CssUpdated':
                    this.hideErrorOverlay();
                    this.updateCss(update);
                    break;

                case 'BuildError':
                    this.showErrorOverlay(update.content || 'Build failed');
                    break;

                case 'BuildSuccess':
                    this.hideErrorOverlay();
                    this.showNotification('✅ Build successful', 'success');
                    break;

                case 'FileAdded':
                case 'FileRemoved':
                    // For now, trigger full reload for file additions/removals
                    this.hideErrorOverlay();
                    this.performFullReload();
                    break;

                default:
                    console.log('[Soku HMR] Unknown update kind:', update.kind);
            }}
        }}

        updateModule(update) {{
            const modulePath = update.path;
            const newContent = update.content;

            if (!newContent) {{
                console.warn('[Soku HMR] No content for module update:', modulePath);
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
                console.warn('[Soku HMR] Module update failed:', error);
                this.performFullReload();
            }}
        }}

        updateCss(update) {{
            const cssPath = update.path;
            const newContent = update.content;

            if (!newContent) {{
                console.warn('[Soku HMR] No content for CSS update:', cssPath);
                return;
            }}

            try {{
                this.hotReplaceCss(cssPath, newContent);
                this.showNotification(`🎨 CSS Updated: ${{cssPath}}`, 'info');
            }} catch (error) {{
                console.warn('[Soku HMR] CSS update failed:', error);
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
            const oldScript = document.querySelector(`script[data-soku-module="${{modulePath}}"]`);
            if (oldScript) {{
                oldScript.remove();
            }}

            // Add new script
            script.setAttribute('data-soku-module', modulePath);
            document.head.appendChild(script);

            // Trigger custom event for application to handle
            window.dispatchEvent(new CustomEvent('soku-hmr-module-updated', {{
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
                    cssElement = document.querySelector(`style[data-soku-css="${{cssPath}}"]`);
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
                style.setAttribute('data-soku-css', cssPath);
                style.textContent = newContent;
                document.head.appendChild(style);
                this.cssLinks.set(cssPath, style);
            }}

            // Trigger custom event
            window.dispatchEvent(new CustomEvent('soku-hmr-css-updated', {{
                detail: {{ cssPath, content: newContent }}
            }}));
        }}

        performFullReload() {{
            this.showNotification('🔄 Reloading page...', 'warning');
            console.log('[Soku HMR] Performing full page reload');

            // Small delay to show notification
            setTimeout(() => {{
                window.location.reload();
            }}, 300);
        }}

        scheduleReconnect() {{
            if (this.reconnectAttempts >= this.maxReconnectAttempts) {{
                console.warn('[Soku HMR] Max reconnection attempts reached');
                this.showNotification('❌ HMR Disconnected', 'error');
                return;
            }}

            this.reconnectAttempts++;
            const delay = this.reconnectDelay * this.reconnectAttempts;

            console.log(`[Soku HMR] Reconnecting in ${{delay}}ms (attempt ${{this.reconnectAttempts}})`);

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

        showErrorOverlay(errorMessage) {{
            // Remove existing overlay
            this.hideErrorOverlay();

            const overlay = document.createElement('div');
            overlay.id = '__soku_hmr_error_overlay__';
            overlay.style.cssText = `
                position: fixed;
                top: 0;
                left: 0;
                width: 100%;
                height: 100%;
                background: rgba(0, 0, 0, 0.9);
                color: #ff5555;
                font-family: 'Menlo', 'Monaco', 'Courier New', monospace;
                font-size: 14px;
                padding: 20px;
                box-sizing: border-box;
                z-index: 999999;
                overflow: auto;
            `;

            const content = document.createElement('div');
            content.style.cssText = `
                max-width: 800px;
                margin: 0 auto;
            `;

            const header = document.createElement('h2');
            header.style.cssText = `
                color: #ff5555;
                margin: 0 0 20px 0;
                font-size: 24px;
                font-weight: bold;
            `;
            header.textContent = '❌ Build Failed';

            const message = document.createElement('pre');
            message.style.cssText = `
                white-space: pre-wrap;
                word-wrap: break-word;
                background: rgba(255, 255, 255, 0.05);
                padding: 15px;
                border-radius: 4px;
                border-left: 4px solid #ff5555;
                margin: 0;
                line-height: 1.5;
            `;
            message.textContent = errorMessage;

            const tip = document.createElement('p');
            tip.style.cssText = `
                margin: 20px 0 0 0;
                color: #888;
                font-size: 12px;
            `;
            tip.textContent = '💡 Fix the error and save the file to continue';

            content.appendChild(header);
            content.appendChild(message);
            content.appendChild(tip);
            overlay.appendChild(content);
            document.body.appendChild(overlay);
        }}

        hideErrorOverlay() {{
            const overlay = document.getElementById('__soku_hmr_error_overlay__');
            if (overlay) {{
                overlay.remove();
            }}
        }}

        setupErrorHandling() {{
            // Handle unhandled promise rejections
            window.addEventListener('unhandledrejection', (event) => {{
                console.warn('[Soku HMR] Unhandled promise rejection:', event.reason);
                const errorMsg = event.reason?.stack || event.reason?.message || String(event.reason);
                this.showErrorOverlay(`Unhandled Promise Rejection:\n\n${{errorMsg}}`);
            }});

            // Handle JavaScript errors
            window.addEventListener('error', (event) => {{
                console.warn('[Soku HMR] JavaScript error:', event.error);
                const errorMsg = event.error?.stack || event.message;
                this.showErrorOverlay(`JavaScript Error:\n\n${{errorMsg}}`);
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
        window.__SOKU_HMR__ = new SokuHMRClient();

        // Expose some utilities for debugging
        window.__SOKU_HMR_DEBUG__ = {{
            reload: () => window.__SOKU_HMR__.performFullReload(),
            stats: () => window.__SOKU_HMR__.getStats(),
            isConnected: () => window.__SOKU_HMR__.isConnected()
        }};

        console.log('🔥 Soku HMR Client initialized');
    }}
}})();
"#,
        port = port
    )
}
