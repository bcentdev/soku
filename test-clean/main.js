// Simple test with lodash
import { map, filter } from 'lodash';

const numbers = [1, 2, 3, 4, 5];
const doubled = map(numbers, x => x * 2);
const evens = filter(numbers, x => x % 2 === 0);

console.log('Doubled:', doubled);
console.log('Evens:', evens);

export { doubled, evens };