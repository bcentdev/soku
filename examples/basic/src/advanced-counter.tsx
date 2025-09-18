// Advanced React TSX component to test oxc transformation
import React, { useState, useEffect, useCallback } from 'react';
import { CounterState, PerformanceTracker, Direction } from './types';

interface AdvancedCounterProps {
    initialValue?: number;
    step?: number;
    onUpdate?: (state: CounterState) => void;
    className?: string;
}

export const AdvancedCounter: React.FC<AdvancedCounterProps> = ({
    initialValue = 0,
    step = 1,
    onUpdate,
    className = 'advanced-counter'
}) => {
    const [state, setState] = useState<CounterState>({
        count: initialValue,
        lastUpdate: Date.now(),
        isEven: initialValue % 2 === 0
    });

    const [tracker] = useState(() => new PerformanceTracker());

    // Memoized callbacks for performance
    const increment = useCallback(() => {
        tracker.mark('increment-start');

        setState(prev => {
            const newCount = prev.count + step;
            const newState: CounterState = {
                count: newCount,
                lastUpdate: Date.now(),
                isEven: newCount % 2 === 0
            };

            onUpdate?.(newState);
            tracker.mark('increment-end');
            return newState;
        });
    }, [step, onUpdate, tracker]);

    const decrement = useCallback(() => {
        tracker.mark('decrement-start');

        setState(prev => {
            const newCount = Math.max(0, prev.count - step);
            const newState: CounterState = {
                count: newCount,
                lastUpdate: Date.now(),
                isEven: newCount % 2 === 0
            };

            onUpdate?.(newState);
            tracker.mark('decrement-end');
            return newState;
        });
    }, [step, onUpdate, tracker]);

    const reset = useCallback(() => {
        setState({
            count: initialValue,
            lastUpdate: Date.now(),
            isEven: initialValue % 2 === 0
        });
    }, [initialValue]);

    // Keyboard handling
    useEffect(() => {
        const handleKeyPress = (event: KeyboardEvent) => {
            switch (event.key) {
                case 'ArrowUp':
                case '+':
                    event.preventDefault();
                    increment();
                    break;
                case 'ArrowDown':
                case '-':
                    event.preventDefault();
                    decrement();
                    break;
                case 'r':
                case 'R':
                    if (event.ctrlKey || event.metaKey) {
                        event.preventDefault();
                        reset();
                    }
                    break;
            }
        };

        window.addEventListener('keydown', handleKeyPress);
        return () => window.removeEventListener('keydown', handleKeyPress);
    }, [increment, decrement, reset]);

    // Performance monitoring
    useEffect(() => {
        console.log('Performance measurements:', tracker.getAllMeasurements());
    }, [state.count, tracker]);

    const getCounterStyle = (): React.CSSProperties => ({
        backgroundColor: state.isEven ? '#e3f2fd' : '#fff3e0',
        border: `2px solid ${state.isEven ? '#2196f3' : '#ff9800'}`,
        borderRadius: '8px',
        padding: '16px',
        transition: 'all 0.3s ease',
        transform: state.count > 10 ? 'scale(1.05)' : 'scale(1)'
    });

    return (
        <div className={className} style={getCounterStyle()}>
            <div className="counter-display">
                <h2>Advanced Counter</h2>
                <div className="count-value">
                    Count: <span style={{
                        fontSize: '2em',
                        color: state.isEven ? '#2196f3' : '#ff9800'
                    }}>
                        {state.count}
                    </span>
                </div>

                <div className="counter-info">
                    <p>Type: {state.isEven ? 'Even' : 'Odd'}</p>
                    <p>Step: {step}</p>
                    <p>Last Update: {new Date(state.lastUpdate).toLocaleTimeString()}</p>
                </div>
            </div>

            <div className="counter-controls">
                <button
                    onClick={decrement}
                    disabled={state.count <= 0}
                    className="btn btn-decrement"
                    title="Decrease (↓ or -)"
                >
                    -
                </button>

                <button
                    onClick={reset}
                    className="btn btn-reset"
                    title="Reset (Ctrl+R)"
                >
                    Reset
                </button>

                <button
                    onClick={increment}
                    className="btn btn-increment"
                    title="Increase (↑ or +)"
                >
                    +
                </button>
            </div>

            <div className="counter-stats">
                <small>
                    Keyboard shortcuts: ↑/+ to increment, ↓/- to decrement, Ctrl+R to reset
                </small>
            </div>
        </div>
    );
};

// HOC example with generics
export function withPerformanceTracking<P extends object>(
    Component: React.ComponentType<P>
) {
    return React.forwardRef<any, P>((props, ref) => {
        const [tracker] = useState(() => new PerformanceTracker());

        useEffect(() => {
            tracker.mark('component-mount');
            return () => {
                tracker.mark('component-unmount');
                console.log('Component lifecycle:', tracker.getAllMeasurements());
            };
        }, [tracker]);

        return <Component {...props} ref={ref} />;
    });
}

export default AdvancedCounter;