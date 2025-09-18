/**
 * Módulo contador para demostrar HMR
 */

export function createCounter() {
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
export function getCounterStats(currentValue) {
    return {
        value: currentValue,
        isEven: currentValue % 2 === 0,
        isPowerOfTwo: currentValue > 0 && (currentValue & (currentValue - 1)) === 0,
        timestamp: Date.now()
    };
}