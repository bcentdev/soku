// Simple test main file
import { add, multiply } from './src/simple.js';

console.log('Testing source maps!');

const result1 = add(5, 3);
const result2 = multiply(4, 6);

console.log('Results:', result1, result2);