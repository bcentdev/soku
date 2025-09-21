// Ultra Bundler - Optimized Build Output
(function() {
'use strict';

// Module: ./esbuild.config.js

const buildEsbuild = async () => {
  const startTime = performance.now()

  try {
    // Create output directory
    mkdirSync('./dist-esbuild', { recursive: true })

    // Build JavaScript
    const jsResult = await build({
      entryPoints: ['./main.js'],
      bundle: true,
      minify: true,
      format: 'esm',
      target: 'es2020',
      outdir: './dist-esbuild',
      write: true,
      metafile: true
    })

    // Build CSS separately (esbuild can't auto-discover CSS from JS imports without explicit imports)
    const cssFiles = ['./styles.css', './styles.module.css', './advanced-styles.css']
    let bundledCSS = '/* esbuild CSS bundle */\n'

    for (const cssFile of cssFiles) {
      try {
        const cssResult = await build({
          entryPoints: [cssFile],
          bundle: true,
          minify: true,
          write: false,
          loader: { '.css': 'css' }
        })

        if (cssResult.outputFiles && cssResult.outputFiles[0]) {
          bundledCSS += `/* From: ${cssFile} */\n`
          bundledCSS += cssResult.outputFiles[0].text + '\n'
        }
      } catch (error) {
        console.warn(`Warning: Could not process ${cssFile}:`, error.message)
      }
    }

    // Write bundled CSS
    writeFileSync('./dist-esbuild/bundle.css', bundledCSS)

    // Copy and update HTML
    let htmlContent = readFileSync('./index.html', 'utf8')
    htmlContent = htmlContent
      .replace('./main.js', './main.js')
      .replace('./styles.css', './bundle.css')

    writeFileSync('./dist-esbuild/index.html', htmlContent)

    const buildTime = performance.now() - startTime

    console.log(`📊 esbuild Statistics:`)
    console.log(`  • Build time: ${buildTime.toFixed(2)}ms`)
    console.log(`  • JS modules: ${Object.keys(jsResult.metafile.inputs).length}`)
    console.log(`  • CSS files: ${cssFiles.length}`)

    return { buildTime, success: true }

  } catch (error) {
    console.error('esbuild failed:', error)
    return { buildTime: performance.now() - startTime, success: false }
  }
}

buildEsbuild()

// Module: ./test-oxc.js
// Test script to verify oxc integration

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

// Module: ./vite.config.js

  build: {
    rollupOptions: {
      input: {
        main: './index.html'
      }
    },
    minify: true,
    outDir: './dist-vite',
    emptyOutDir: true
  }
})

// Module: ./main.js

// Estado de la aplicación
let state = {
    count: 0,
    lastUpdate: Date.now()
};

// Elementos del DOM
const countElement = document.getElementById('count');
const statusElement = document.getElementById('status');
const incrementBtn = document.getElementById('increment');
const decrementBtn = document.getElementById('decrement');

// Crear contador
const counter = createCounter();

// Funciones de actualización
function updateUI() {
    countElement.textContent = state.count;
    statusElement.textContent = `Última actualización: ${new Date(state.lastUpdate).toLocaleTimeString()}`;

    // Log para demostrar HMR
    logger.info(`Count updated to: ${state.count}`);
}

function increment() {
    state.count = counter.increment(state.count);
    state.lastUpdate = Date.now();
    updateUI();
}

function decrement() {
    state.count = counter.decrement(state.count);
    state.lastUpdate = Date.now();
    updateUI();
}

// Event listeners
incrementBtn.addEventListener('click', increment);
decrementBtn.addEventListener('click', decrement);

// Inicialización
updateUI();

// HMR API para hot reloading
if (import.meta.hot) {
    import.meta.hot.accept('./counter.js', (newCounter) => {
        console.log('🔥 Counter module updated!');
        // Actualizar la referencia al módulo
        Object.assign(counter, newCounter.createCounter());
    });

    import.meta.hot.accept('./utils/logger.js', () => {
        console.log('🔥 Logger module updated!');
    });
}

console.log('🚀 Ultra Bundler app initialized!');

// Module: ./main-rspack.js
// Import CSS for rspack

// Import main functionality

// Estado de la aplicación
let state = {
    count: 0,
    lastUpdate: Date.now()
};

// Elementos del DOM
const countElement = document.getElementById('count');
const statusElement = document.getElementById('status');
const incrementBtn = document.getElementById('increment');
const decrementBtn = document.getElementById('decrement');

// Crear contador
const counter = createCounter();

// Funciones de actualización
function updateUI() {
    countElement.textContent = state.count;
    statusElement.textContent = `Última actualización: ${new Date(state.lastUpdate).toLocaleTimeString()}`;

    // Log para demostrar HMR
    logger.info(`Count updated to: ${state.count}`);
}

function increment() {
    state.count = counter.increment(state.count);
    state.lastUpdate = Date.now();
    updateUI();
}

function decrement() {
    state.count = counter.decrement(state.count);
    state.lastUpdate = Date.now();
    updateUI();
}

// Event listeners
incrementBtn.addEventListener('click', increment);
decrementBtn.addEventListener('click', decrement);

// Inicialización
updateUI();

// HMR API para hot reloading
if (import.meta.hot) {
    import.meta.hot.accept('./counter.js', (newCounter) => {
        console.log('🔥 Counter module updated!');
        // Actualizar la referencia al módulo
        Object.assign(counter, newCounter.createCounter());
    });

    import.meta.hot.accept('./utils/logger.js', () => {
        console.log('🔥 Logger module updated!');
    });
}

console.log('🚀 Ultra Bundler app initialized!');

// Module: ./main-tree-shaking.js
// Tree shaking demo entry point
console.log("Tree shaking demo starting!");

// Import only some functions - others should be tree-shaken

// Note: unusedFunction and completelyUnusedFunction should be removed

function main() {
    console.log("Starting tree shaking demo");

    // Use some imports - these should be included
    console.log(usedFunction());
    console.log(anotherUsedFunction(5));
    console.log(defaultHelper());

    console.log("Tree shaking demo completed");
}

main();

// Module: ./counter.js
/**
 * Módulo contador para demostrar HMR
 */

    return {
        increment: (value) => {
            console.log(`Incrementing ${value}`);
            return value + 1;
        },

        decrement: (value) => {
            console.log(`Decrementing ${value}`);
            return Math.max(0, value - 1); // No permite valores negativos
        },

        reset: () => {
            console.log('Resetting counter');
            return 0;
        }
    };
}

// Función auxiliar para debugging
    return {
        value: currentValue,
        isEven: currentValue % 2 === 0,
        isPowerOfTwo: currentValue > 0 && (currentValue & (currentValue - 1)) === 0,
        timestamp: Date.now()
    };
}

// Module: ./utils.js
// utilities module with tree shaking demo

    console.log("This function is used and should be included");
    return "used result";
};

    console.log("This function is never used and should be tree-shaken");
    return "unused result";
};

    return value * 2;
}

    console.log("This is completely unused");
    return "waste of space";
}

// Default export that is used
    return "default helper result";
}

})();
