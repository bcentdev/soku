// Test WASM loading
import { mathModule } from './math.wasm';

async function run() {
  const wasm = await mathModule.load();
  console.log('WASM module loaded!', wasm);
  
  // Example: Call WASM function
  if (wasm.add) {
    const result = wasm.add(5, 3);
    console.log('5 + 3 =', result);
  }
}

run().catch(console.error);
