// Ultra Bundler Demo Project
import { createApp } from './src/app.js';
import { utils } from './src/utils.js';
import { createUser, UserManager } from './src/types.ts';
import './src/styles.css';

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

// Test TypeScript functionality
const testUser = createUser('John Doe', 'john@example.com');
console.log('🧑 Created user:', testUser);

const userManager = new UserManager('https://api.example.com/users');
console.log('👥 User manager initialized');

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