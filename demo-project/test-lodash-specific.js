// Test specific lodash functions
import { map, filter, reduce } from 'lodash';

const numbers = [1, 2, 3, 4, 5, 6];

// Test lodash functions
const doubled = map(numbers, n => n * 2);
const evens = filter(numbers, n => n % 2 === 0);
const sum = reduce(numbers, (acc, n) => acc + n, 0);

console.log('Doubled:', doubled);
console.log('Evens:', evens);
console.log('Sum:', sum);

export { doubled, evens, sum };