// Clean node modules test
import { map, filter, sum } from 'simple-utils';
import utils from 'simple-utils';

console.log('Testing Ultra Bundler with clean node_modules!');

const numbers = [1, 2, 3, 4, 5];
const doubled = map(numbers, n => n * 2);
const evens = filter(numbers, n => n % 2 === 0);
const cloned = utils.clone({ a: 1, b: 2 });
const total = sum(10, 5);

console.log('Doubled:', doubled);
console.log('Evens:', evens);
console.log('Cloned:', cloned);
console.log('Sum 10 + 5 =', total);