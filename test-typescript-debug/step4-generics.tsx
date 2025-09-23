// Step 4: Test generics (this was failing)
interface ListProps<T> {
    items: T[];
    renderItem: (item: T) => JSX.Element;
}

export function List<T>({ items, renderItem }: ListProps<T>) {
    return (
        <ul>
            {items.map((item, index) => (
                <li key={index}>
                    {renderItem(item)}
                </li>
            ))}
        </ul>
    );
}