import { A } from './cases';
export let receiver = new A();
export let /*@importPlainDef*/cell = 0;
export function reset() { receiver = new A(); cell = 0; }
