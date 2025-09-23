// Complex combo: generics + array destructuring + optional chaining + JSX
interface Props {
    onCountChange?: (count: number) => void;
}

function useState<T>(initialValue: T): [T, (value: T) => void] {
    let value = initialValue;
    const setValue = (newValue: T) => {
        value = newValue;
    };
    return [value, setValue];
}

export const ComplexComponent = ({ onCountChange }: Props) => {
    const [count, setCount] = useState(0);

    const handleIncrement = () => {
        const newCount = count + 1;
        setCount(newCount);
        onCountChange?.(newCount);
    };

    return (
        <div>
            <span>{count}</span>
            <button onClick={handleIncrement}>+</button>
        </div>
    );
};