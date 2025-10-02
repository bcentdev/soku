// Calculator with TypeScript types
export class Calculator {
    private history: number[] = [];

    add(a: number, b: number): number {
        const result = a + b;
        this.history.push(result);
        return result;
    }

    multiply(a: number, b: number): number {
        return a * b;
    }

    getHistory(): number[] {
        return this.history;
    }
}
