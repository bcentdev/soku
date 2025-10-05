// Clean test project
import { utils } from './utils.js';
import { components } from './components.js';
import './styles.css';

console.log('🚀 Clean Soku Build Test');

// Initialize utilities
utils.init();

// Render components
components.render();

// Test functionality
const result = utils.processData({
    name: 'Soku Bundler',
    version: '0.3.0',
    features: ['Fast', 'TypeScript', 'Tree Shaking']
});

console.log('Processed data:', result);

export { utils, components };