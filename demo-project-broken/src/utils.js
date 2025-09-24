// Utility functions - Fixed
export const utils = {
    init() {
        console.log('Utils initialized');
        this.startTime = Date.now();
    },

    formatData(data) {
        return {
            ...data,
            timestamp: new Date().toISOString(),
            processingTime: Date.now() - this.startTime
        };
    },

    debounce(func, delay) {
        let timeoutId;
        return function(...args) {
            clearTimeout(timeoutId);
            timeoutId = setTimeout(() => func.apply(this, args), delay);
        };
    },

    throttle(func, limit) {
        let inThrottle;
        return function() {
            const args = arguments;
            const context = this;
            if (!inThrottle) {
                func.apply(context, args);
                inThrottle = true;
                setTimeout(() => inThrottle = false, limit);
            }
        };
    }
};