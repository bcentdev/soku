interface User {
  name: string;
  age: number;
}

const user: User = {
  name: 'John',
  age: 30
};

function greetUser(user: User): string {
  return `Hello, ${user.name}!`;
}

export { user, greetUser };