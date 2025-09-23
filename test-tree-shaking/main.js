// Test TypeScript transformation with oxc 0.90
import { user, greetUser } from './typescript-test.ts';

console.log('Testing TypeScript transformation with oxc 0.90...');
console.log(user.name, user.age);
console.log(greetUser(user));