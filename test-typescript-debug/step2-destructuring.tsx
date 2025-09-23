// Step 2: Test destructuring with types
interface Props {
    text: string;
    onClick: () => void;
}

export const Button = ({ text, onClick }: Props) => {
    return <button onClick={onClick}>{text}</button>;
};