// Utility functions for Ultra Bundler Demo
export const utils = {
    formatData(data) {
        return {
            ...data,
            timestamp: new Date().toISOString(),
            formatted: true
        };
    },

    debounce(func, wait) {
        let timeout;
        return function executedFunction(...args) {
            const later = () => {
                clearTimeout(timeout);
                func(...args);
            };
            clearTimeout(timeout);
            timeout = setTimeout(later, wait);
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
    },

    deepClone(obj) {
        if (obj === null || typeof obj !== "object") {
            return obj;
        }
        if (obj instanceof Date) {
            return new Date(obj.getTime());
        }
        if (obj instanceof Array) {
            return obj.map(item => this.deepClone(item));
        }
        if (typeof obj === "object") {
            const copy = {};
            Object.keys(obj).forEach(key => {
                copy[key] = this.deepClone(obj[key]);
            });
            return copy;
        }
    },

    generateId() {
        return Math.random().toString(36).substr(2, 9);
    },

    log(level, message, data = null) {
        const timestamp = new Date().toISOString();
        const logEntry = {
            timestamp,
            level,
            message,
            data
        };

        switch (level) {
            case 'info':
                console.log('ℹ️', message, data || '');
                break;
            case 'warn':
                console.warn('⚠️', message, data || '');
                break;
            case 'error':
                console.error('❌', message, data || '');
                break;
            default:
                console.log('📝', message, data || '');
        }

        return logEntry;
    }
};