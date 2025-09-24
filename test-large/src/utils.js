// Utility functions
export const utils = {
    init() {
        console.log('Utils initialized');
        this.setupHelpers();
    },

    setupHelpers() {
        this.debounce = (func, wait) => {
            let timeout;
            return function executedFunction(...args) {
                const later = () => {
                    clearTimeout(timeout);
                    func(...args);
                };
                clearTimeout(timeout);
                timeout = setTimeout(later, wait);
            };
        };

        this.throttle = (func, limit) => {
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
        };
    },

    formatDate(date) {
        return new Intl.DateTimeFormat('en-US').format(date);
    },

    generateId() {
        return Math.random().toString(36).substr(2, 9);
    }
};