// Entry point
import { Simple } from './step1-simple.tsx';
import { Button as Button2 } from './step2-destructuring.tsx';
import { Button as Button3 } from './step3-defaults.tsx';

import { List } from './step4-generics.tsx';
import { withLoading } from './step5-intersection.tsx';
import { useState } from './step6-tuples.tsx';
import { TestOptional } from './test-only-optional.tsx';

console.log('Testing optional chaining...');
console.log(TestOptional);