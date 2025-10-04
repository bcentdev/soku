// User management with TypeScript types
export interface User {
    name: string;
    age: number;
    email?: string;
}

export function createUser(name: string, age: number, email?: string): User {
    return { name, age, email };
}

export function isAdult(user: User): boolean {
    return user.age >= 18;
}
