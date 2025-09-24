// Ultra Bundler Demo - Fixed
import { utils } from './src/utils.js';
import { components } from './src/app.js';
import './src/styles.css';

console.log('🚀 Ultra Bundler Demo Starting...');

// Initialize utilities
utils.init();

// Render components
components.render();

// Test functionality
const result = utils.formatData({
    name: 'Ultra Bundler',
    version: '0.3.0',
    performance: '35x faster'
});

console.log('Demo result:', result);

export { utils, components };