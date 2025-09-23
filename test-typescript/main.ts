// Simple TypeScript test
interface User {
    name: string;
    age: number;
}

export function createUser(name: string, age: number): User {
    return { name, age };
}

const user: User = createUser('Alice', 30);
console.log('User created:', user);