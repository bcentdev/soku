// TypeScript example to test oxc integration

export interface CounterState {
    count: number;
    lastUpdate: number;
    isEven: boolean;
}

export interface LogLevel {
    DEBUG: 0;
    INFO: 1;
    WARN: 2;
    ERROR: 3;
}

export type LogMessage = {
    level: keyof LogLevel;
    message: string;
    timestamp: Date;
};

export class PerformanceTracker {
    private startTime: number;
    private measurements: Map<string, number> = new Map();

    constructor() {
        this.startTime = performance.now();
    }

    mark(label: string): void {
        this.measurements.set(label, performance.now() - this.startTime);
    }

    getMeasurement(label: string): number | undefined {
        return this.measurements.get(label);
    }

    getAllMeasurements(): Record<string, number> {
        return Object.fromEntries(this.measurements);
    }
}

// Generic utility types
export type Optional<T> = T | undefined;
export type Nullable<T> = T | null;
export type Maybe<T> = T | null | undefined;

// Function type for counter operations
export type CounterOperation = (current: number) => number;

// React-like component props
export interface ComponentProps {
    children?: React.ReactNode;
    className?: string;
    onClick?: (event: MouseEvent) => void;
}

// Advanced TypeScript features
export type DeepReadonly<T> = {
    readonly [P in keyof T]: T[P] extends object ? DeepReadonly<T[P]> : T[P];
};

export type EventHandler<T = Event> = (event: T) => void | Promise<void>;

// Module augmentation example
declare global {
    interface Window {
        __ULTRA_BUNDLER_DEV__: boolean;
        __ULTRA_HMR__: {
            accept: (callback?: Function) => void;
            decline: () => void;
        };
    }
}

export const enum Direction {
    Up = "UP",
    Down = "DOWN",
    Left = "LEFT",
    Right = "RIGHT",
}

// Example of complex type manipulation
export type ExtractArrayType<T> = T extends (infer U)[] ? U : never;
export type FunctionReturnType<T> = T extends (...args: any[]) => infer R ? R : never;