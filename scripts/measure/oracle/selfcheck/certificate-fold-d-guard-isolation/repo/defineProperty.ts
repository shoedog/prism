declare const module: { exports: object };
export function f(/*@defineProperty-formal*/formal: number) { return formal; }
Object.defineProperty(module.exports, 'marker', { value: 1 });
