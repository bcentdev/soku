// Test only optional chaining
export const TestOptional = (callback) => {
    const value = 42;
    callback?.(value);
    return value;
};