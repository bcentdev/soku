import { createCounter } from './counter.js';
import { logger } from './utils/logger.js';

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