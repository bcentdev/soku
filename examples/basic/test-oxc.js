// Test script to verify oxc integration
import { createCounter } from './counter.js';
import { logger } from './utils/logger.js';

// Modern JavaScript features to test oxc parsing
const config = {
    version: '1.0.0',
    features: ['hmr', 'fast-refresh', 'typescript'],
    performance: {
        target: 'sub-50ms',
        baseline: 'vite'
    }
};

// Async/await
async function initializeApp() {
    logger.info('🚀 Initializing Ultra Bundler test app...');

    const counter = createCounter();
    let count = 0;

    // Object destructuring and spread
    const { increment, decrement } = counter;
    const newConfig = { ...config, timestamp: Date.now() };

    // Template literals
    console.log(`App config: ${JSON.stringify(newConfig, null, 2)}`);

    // Array methods and arrow functions
    const operations = [
        () => count = increment(count),
        () => count = increment(count),
        () => count = decrement(count),
    ];

    // Execute operations
    operations.forEach(op => op());

    // Dynamic import test
    try {
        const { PerformanceTracker } = await import('./src/types.ts');
        const tracker = new PerformanceTracker();
        tracker.mark('app-initialized');

        logger.info(`Final count: ${count}`);
        logger.info('Performance:', tracker.getAllMeasurements());
    } catch (error) {
        logger.error('Failed to load TypeScript module:', error);
    }

    return { count, config: newConfig };
}

// Optional chaining and nullish coalescing
const getConfigValue = (key) => config?.performance?.[key] ?? 'unknown';

// Class with private fields
class TestRunner {
    #results = new Map();
    #startTime = performance.now();

    async runTest(name, testFn) {
        const start = performance.now();
        try {
            await testFn();
            this.#results.set(name, {
                status: 'passed',
                duration: performance.now() - start
            });
            logger.info(`✅ Test passed: ${name}`);
        } catch (error) {
            this.#results.set(name, {
                status: 'failed',
                error: error.message,
                duration: performance.now() - start
            });
            logger.error(`❌ Test failed: ${name} - ${error.message}`);
        }
    }

    getResults() {
        return Object.fromEntries(this.#results);
    }

    getTotalDuration() {
        return performance.now() - this.#startTime;
    }
}

// Export for HMR testing
export { initializeApp, TestRunner, getConfigValue };

// Auto-initialize if this is the main module
if (import.meta.hot) {
    import.meta.hot.accept((newModule) => {
        console.log('🔥 test-oxc.js reloaded!');
        // Re-run initialization
        newModule.initializeApp().then(result => {
            console.log('Hot reload complete:', result);
        });
    });

    // Run tests on load
    const runner = new TestRunner();

    runner.runTest('initialization', async () => {
        const result = await initializeApp();
        if (result.count !== 1) {
            throw new Error(`Expected count 1, got ${result.count}`);
        }
    });

    runner.runTest('config-access', () => {
        const target = getConfigValue('target');
        if (target !== 'sub-50ms') {
            throw new Error(`Expected 'sub-50ms', got '${target}'`);
        }
    });

    setTimeout(() => {
        logger.info('Test Results:', runner.getResults());
        logger.info(`Total test duration: ${runner.getTotalDuration()}ms`);
    }, 100);
}