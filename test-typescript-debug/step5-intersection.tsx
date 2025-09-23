// Step 5: Test intersection types (likely culprit)
export function withLoading<T>(Component: any) {
    return (props: T & { isLoading?: boolean }) => {
        if (props.isLoading) {
            return <div>Loading...</div>;
        }
        return <Component {...props} />;
    };
}