// Main entry point for large project
import { utils } from './src/utils.js';
import { components } from './src/components.js';
import { services } from './src/services.js';
import { types } from './src/types.ts';
import { helpers } from './src/helpers.js';
import { getConfig } from './src/config.js';
import './src/styles.css';

console.log('Large project loading...');

// Get configuration
const config = getConfig('development');
console.log('Config:', config);

// Initialize utilities
utils.init();
components.render();
services.start();

// Test helpers
const userId = helpers.generateId();
console.log('Generated ID:', userId);

// Test types
const dataManager = new types.DataManager(config.apiUrl);
console.log('Data manager initialized');

export { utils, components, services, types, helpers, config };