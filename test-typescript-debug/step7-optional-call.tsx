// Step 7: Test optional chaining call (the real culprit!)
interface Props {
    onCallback?: (value: number) => void;
}

export const Test = ({ onCallback }: Props) => {
    const handleClick = () => {
        const value = 42;
        onCallback?.(value); // This line is causing the problem!
    };

    return <button onClick={handleClick}>Click me</button>;
};