// Test file for node_modules tree shaking
import { map, filter } from 'lodash';
import _ from 'lodash';
import * as utils from 'lodash';

// Only use some functions
const numbers = [1, 2, 3, 4, 5];
const doubled = map(numbers, n => n * 2);
const evens = filter(numbers, n => n % 2 === 0);

// Use default import
const cloned = _.clone({ a: 1, b: 2 });

// Use namespace import
const sorted = utils.sortBy([{name: 'b'}, {name: 'a'}], 'name');

console.log('Doubled:', doubled);
console.log('Evens:', evens);
console.log('Cloned:', cloned);
console.log('Sorted:', sorted);

export default function testNodeModules() {
    return { doubled, evens, cloned, sorted };
}