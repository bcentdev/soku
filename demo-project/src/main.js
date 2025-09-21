// Ultra Bundler Demo Project
import { createApp } from './app.js';
import { utils } from './utils.js';
import './styles.css';

console.log('🚀 Ultra Bundler Demo Starting...');

// Initialize the application
const app = createApp({
    title: 'Ultra Fast Demo',
    version: '1.0.0',
    features: ['HMR', 'Tree Shaking', 'Lightning CSS', 'TypeScript Support']
});

// Use utilities
const formattedData = utils.formatData({
    buildTime: '0.51ms',
    performance: '35x faster than esbuild',
    architecture: 'Clean Architecture with Rust'
});

console.log('📊 Performance Data:', formattedData);

// DOM manipulation
document.addEventListener('DOMContentLoaded', () => {
    const root = document.getElementById('app');
    if (root) {
        root.innerHTML = app.render();

        // Add interactivity
        const button = root.querySelector('#demo-button');
        if (button) {
            button.addEventListener('click', () => {
                app.handleClick();
            });
        }
    }
});

// Export for potential HMR
export { app, utils };