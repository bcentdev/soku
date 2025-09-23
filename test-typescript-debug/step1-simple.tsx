// Step 1: Simple interface
interface Props {
    text: string;
}

export const Simple = (props: Props) => {
    return <div>{props.text}</div>;
};