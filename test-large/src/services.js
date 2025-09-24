// Service layer
export const services = {
    async start() {
        console.log('Services starting');
        await this.initDatabase();
        await this.setupAPI();
        this.startPolling();
    },

    async initDatabase() {
        return new Promise(resolve => {
            setTimeout(() => {
                console.log('Database initialized');
                resolve();
            }, 100);
        });
    },

    async setupAPI() {
        this.apiEndpoint = 'https://api.example.com';
        console.log('API configured:', this.apiEndpoint);
    },

    startPolling() {
        setInterval(() => {
            this.fetchData();
        }, 5000);
    },

    async fetchData() {
        try {
            const response = await fetch(this.apiEndpoint + '/data');
            const data = await response.json();
            console.log('Data fetched:', data);
        } catch (error) {
            console.warn('Failed to fetch data:', error);
        }
    }
};