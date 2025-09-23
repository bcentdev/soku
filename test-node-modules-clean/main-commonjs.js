// Testing CommonJS support
import utils from 'commonjs-utils';
import { map, filter } from 'simple-utils';

console.log('Testing Ultra Bundler with CommonJS and ES6 modules!');

// Test CommonJS functions
const result = utils.multiply(5, 3);
const quotient = utils.divide(10, 2);
const title = utils.capitalize('hello world');

// Test ES6 functions
const numbers = [1, 2, 3, 4, 5];
const doubled = map(numbers, n => n * 2);
const evens = filter(numbers, n => n % 2 === 0);

console.log('CommonJS multiply 5 * 3 =', result);
console.log('CommonJS divide 10 / 2 =', quotient);
console.log('CommonJS capitalize =', title);
console.log('ES6 map doubled =', doubled);
console.log('ES6 filter evens =', evens);
console.log('CommonJS utils version =', utils.version);