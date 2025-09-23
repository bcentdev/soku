// Advanced TypeScript features test

interface ApiResponse<T> {
    data: T;
    status: number;
    message: string;
}

type UserRole = 'admin' | 'user' | 'moderator';

interface UserConfig {
    name: string;
    role: UserRole;
    permissions?: string[];
}

// Generic function with constraints
function processApiResponse<T extends object>(response: ApiResponse<T>): T {
    if (response.status !== 200) {
        throw new Error(response.message);
    }
    return response.data;
}

// Class with generics
class DataStore<T> {
    private data: Map<string, T> = new Map();

    set(key: string, value: T): void {
        this.data.set(key, value);
    }

    get(key: string): T | undefined {
        return this.data.get(key);
    }
}

// Arrow functions with complex typing
const userValidator = (config: UserConfig): boolean => {
    return config.name.length > 0 && ['admin', 'user', 'moderator'].includes(config.role);
};

const store = new DataStore<UserConfig>();
const testConfig: UserConfig = {
    name: "Alice",
    role: "admin",
    permissions: ["read", "write"]
};

store.set("user1", testConfig);
console.log("User valid:", userValidator(testConfig));

export { DataStore, processApiResponse, userValidator };
export type { ApiResponse, UserRole, UserConfig };