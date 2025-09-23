// Step 6: Test tuple return types
function useState<T>(initialValue: T): [T, (value: T) => void] {
    let value = initialValue;
    const setValue = (newValue: T) => {
        value = newValue;
    };
    return [value, setValue];
}

export { useState };