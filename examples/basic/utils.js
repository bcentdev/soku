// utilities module with tree shaking demo

export const usedFunction = () => {
    console.log("This function is used and should be included");
    return "used result";
};

export const unusedFunction = () => {
    console.log("This function is never used and should be tree-shaken");
    return "unused result";
};

export function anotherUsedFunction(value) {
    return value * 2;
}

export function completelyUnusedFunction() {
    console.log("This is completely unused");
    return "waste of space";
}

// Default export that is used
export default function defaultHelper() {
    return "default helper result";
}