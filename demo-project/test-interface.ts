// Test file with just interface

interface User {
    name: string;
    age: number;
}

const user: User = {
    name: "Alice",
    age: 30
};

console.log(user);