// Tree shaking demo entry point
console.log("Tree shaking demo starting!");

// Import only some functions - others should be tree-shaken
import { usedFunction, anotherUsedFunction } from './utils.js';
import defaultHelper from './utils.js';

// Note: unusedFunction and completelyUnusedFunction should be removed

function main() {
    console.log("Starting tree shaking demo");

    // Use some imports - these should be included
    console.log(usedFunction());
    console.log(anotherUsedFunction(5));
    console.log(defaultHelper());

    console.log("Tree shaking demo completed");
}

main();