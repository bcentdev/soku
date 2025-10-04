// Utility functions
export const utils = {
    init() {
        console.log('Utils initialized');
        this.startTime = Date.now();
    },

    processData(data) {
        return {
            ...data,
            processedAt: new Date().toISOString(),
            processingTime: Date.now() - this.startTime
        };
    },

    formatSize(bytes) {
        const units = ['B', 'KB', 'MB', 'GB'];
        let size = bytes;
        let unit = 0;

        while (size >= 1024 && unit < units.length - 1) {
            size /= 1024;
            unit++;
        }

        return `${size.toFixed(1)} ${units[unit]}`;
    },

    debounce(func, delay) {
        let timeoutId;
        return function(...args) {
            clearTimeout(timeoutId);
            timeoutId = setTimeout(() => func.apply(this, args), delay);
        };
    }
};