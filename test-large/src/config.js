// Configuration file
export const defaultConfig = {
    apiUrl: 'https://api.example.com',
    timeout: 5000,
    retries: 3,
    debugMode: false
};

export function getConfig(env) {
    if (env === 'development') {
        return {
            apiUrl: 'https://dev-api.example.com',
            timeout: 5000,
            retries: 3,
            debugMode: true
        };
    }

    if (env === 'production') {
        return {
            apiUrl: 'https://api.example.com',
            timeout: 3000,
            retries: 1,
            debugMode: false
        };
    }

    return defaultConfig;
}