declare const module: { exports: object };
export function f(/*@control-formal*/formal: number) { return formal; }
const local = {};
Object.defineProperty(local, 'marker', { value: 1 });
