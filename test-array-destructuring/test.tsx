// Test array destructuring pattern
function useState<T>(initial: T): [T, (value: T) => void] {
    return [initial, () => {}];
}

export const TestDestructuring = () => {
    const [count, setCount] = useState(0);
    return <div>{count}</div>;
};