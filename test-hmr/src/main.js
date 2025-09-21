// Test HMR functionality
console.log('🚀 Ultra HMR Test App v1.0');

const app = document.getElementById('app');
if (app) {
    app.innerHTML = `
        <h1>🔥 Ultra HMR Test</h1>
        <p>This is a test application for Ultra Bundler's Hot Module Replacement.</p>
        <div id="counter">0</div>
        <button onclick="increment()">+</button>
        <button onclick="decrement()">-</button>
    `;
}

let counter = 0;

function increment() {
    counter++;
    updateCounter();
}

function decrement() {
    counter--;
    updateCounter();
}

function updateCounter() {
    const counterEl = document.getElementById('counter');
    if (counterEl) {
        counterEl.textContent = counter;
    }
}

// Make functions global for onclick
window.increment = increment;
window.decrement = decrement;

// HMR event listeners for testing
if (window.__ULTRA_HMR__) {
    window.addEventListener('ultra-hmr-module-updated', (event) => {
        console.log('🔥 Module updated:', event.detail);
    });

    window.addEventListener('ultra-hmr-css-updated', (event) => {
        console.log('🎨 CSS updated:', event.detail);
    });
}