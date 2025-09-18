/**
 * Sistema de logging simple para la demo
 */

const LOG_LEVELS = {
    DEBUG: 0,
    INFO: 1,
    WARN: 2,
    ERROR: 3
};

let currentLevel = LOG_LEVELS.INFO;

function formatMessage(level, message) {
    const timestamp = new Date().toISOString();
    const prefix = {
        [LOG_LEVELS.DEBUG]: '🐛',
        [LOG_LEVELS.INFO]: 'ℹ️',
        [LOG_LEVELS.WARN]: '⚠️',
        [LOG_LEVELS.ERROR]: '❌'
    }[level] || 'ℹ️';

    return `${prefix} [${timestamp}] ${message}`;
}

export const logger = {
    debug: (message) => {
        if (currentLevel <= LOG_LEVELS.DEBUG) {
            console.log(formatMessage(LOG_LEVELS.DEBUG, message));
        }
    },

    info: (message) => {
        if (currentLevel <= LOG_LEVELS.INFO) {
            console.log(formatMessage(LOG_LEVELS.INFO, message));
        }
    },

    warn: (message) => {
        if (currentLevel <= LOG_LEVELS.WARN) {
            console.warn(formatMessage(LOG_LEVELS.WARN, message));
        }
    },

    error: (message) => {
        if (currentLevel <= LOG_LEVELS.ERROR) {
            console.error(formatMessage(LOG_LEVELS.ERROR, message));
        }
    },

    setLevel: (level) => {
        if (typeof level === 'string') {
            currentLevel = LOG_LEVELS[level.toUpperCase()] ?? LOG_LEVELS.INFO;
        } else {
            currentLevel = level;
        }
    }
};

// Para debugging de HMR
logger.info('Logger module loaded!');