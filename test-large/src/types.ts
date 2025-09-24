// Simple TypeScript types
export interface User {
    id: string;
    name: string;
    email: string;
}

export class DataManager {
    constructor(private apiEndpoint: string) {
        this.apiEndpoint = apiEndpoint;
    }

    async fetchUsers() {
        try {
            const response = await fetch(this.apiEndpoint + '/users');
            const users = await response.json();
            return users;
        } catch (error) {
            return [];
        }
    }

    getUserById(id: string) {
        return { id: id, name: 'Test User', email: 'test@example.com' };
    }
}

export const types = {
    DataManager
};

export default DataManager;