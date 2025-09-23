// TypeScript Demo - Types and Interfaces

export interface User {
    id: number;
    name: string;
    email: string;
    isActive: boolean;
    profile?: UserProfile;
}

export interface UserProfile {
    avatar: string;
    bio: string;
    followers: number;
}

export type UserCallback = (user: User) => void;
export type StatusType = 'loading' | 'success' | 'error' | 'idle';

export enum Role {
    ADMIN = 'admin',
    USER = 'user',
    MODERATOR = 'moderator'
}

export const enum Theme {
    LIGHT = 'light',
    DARK = 'dark',
    AUTO = 'auto'
}

// Generic types
export interface ApiResponse<T> {
    data: T;
    status: number;
    message: string;
}

// Function with types
export function createUser(name: string, email: string): User {
    return {
        id: Math.random(),
        name,
        email,
        isActive: true
    };
}

// Arrow function with types
export const validateUser = (user: User): boolean => {
    return user.name.length > 0 && user.email.includes('@');
};

// Class with types
export class UserManager {
    private users: User[] = [];

    constructor(private apiUrl: string) {}

    async fetchUsers(): Promise<User[]> {
        const response = await fetch(this.apiUrl);
        const data: ApiResponse<User[]> = await response.json();
        return data.data;
    }

    addUser(user: User): void {
        this.users.push(user);
    }

    findUser(id: number): User | undefined {
        return this.users.find(u => u.id === id);
    }
}