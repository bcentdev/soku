// Application module for Ultra Bundler Demo
export function createApp(config) {
    let clickCount = 0;

    return {
        config,

        render() {
            return `
                <div class="ultra-demo">
                    <header class="header">
                        <h1>⚡ ${config.title}</h1>
                        <p class="version">Version ${config.version}</p>
                    </header>

                    <main class="main-content">
                        <section class="features">
                            <h2>🔥 Features</h2>
                            <ul class="feature-list">
                                ${config.features.map(feature =>
                                    `<li class="feature-item">${feature}</li>`
                                ).join('')}
                            </ul>
                        </section>

                        <section class="demo-section">
                            <h2>🎯 Interactive Demo</h2>
                            <div class="click-counter">
                                <p>Button clicks: <span id="click-count">${clickCount}</span></p>
                                <button id="demo-button" class="demo-button">
                                    Click me! 🚀
                                </button>
                            </div>
                        </section>

                        <section class="performance">
                            <h2>📊 Performance Stats</h2>
                            <div class="stats-grid">
                                <div class="stat-item">
                                    <div class="stat-value">0.51ms</div>
                                    <div class="stat-label">Build Time</div>
                                </div>
                                <div class="stat-item">
                                    <div class="stat-value">35x</div>
                                    <div class="stat-label">Faster than esbuild</div>
                                </div>
                                <div class="stat-item">
                                    <div class="stat-value">🔥</div>
                                    <div class="stat-label">HMR Ready</div>
                                </div>
                            </div>
                        </section>
                    </main>

                    <footer class="footer">
                        <p>Built with Ultra Bundler - The fastest bundler in the universe</p>
                    </footer>
                </div>
            `;
        },

        handleClick() {
            clickCount++;
            const countElement = document.getElementById('click-count');
            if (countElement) {
                countElement.textContent = clickCount;

                // Add some visual feedback
                countElement.style.transform = 'scale(1.2)';
                setTimeout(() => {
                    countElement.style.transform = 'scale(1)';
                }, 150);
            }

            console.log(`🎯 Button clicked ${clickCount} times`);
        },

        getStats() {
            return {
                clickCount,
                config: this.config
            };
        }
    };
}