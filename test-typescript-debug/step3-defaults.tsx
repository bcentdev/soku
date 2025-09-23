// Step 3: Test default values + optional types
interface Props {
    text: string;
    disabled?: boolean;
    variant?: 'primary' | 'secondary';
}

export const Button = ({ text, disabled = false, variant = 'primary' }: Props) => {
    const className = `btn btn-${variant}`;
    return <button disabled={disabled} className={className}>{text}</button>;
};