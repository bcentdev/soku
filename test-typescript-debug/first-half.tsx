// React-like TSX Component Demo for Ultra Bundler

interface ButtonProps {
    text: string;
    onClick: () => void;
    disabled?: boolean;
    variant?: 'primary' | 'secondary';
}

export const Button = ({ text, onClick, disabled = false, variant = 'primary' }: ButtonProps) => {
    const className = `btn btn-${variant} ${disabled ? 'btn-disabled' : ''}`;

    return (
        <button
            className={className}
            onClick={onClick}
            disabled={disabled}
            type="button"
        >
            {text}
        </button>
    );
};

interface CounterProps {
    initialCount?: number;
    onCountChange?: (count: number) => void;
}

export const Counter = ({ initialCount = 0, onCountChange }: CounterProps) => {
    const [count, setCount] = useState(initialCount);

    const handleIncrement = () => {
        const newCount = count + 1;
        setCount(newCount);
        onCountChange?.(newCount);
    };

    const handleDecrement = () => {
        const newCount = count - 1;
        setCount(newCount);
        onCountChange?.(newCount);
    };

    return (
        <div className="counter">
            <h3>Counter Component</h3>
            <div className="counter-display">
                <span className="count-value">{count}</span>
            </div>
