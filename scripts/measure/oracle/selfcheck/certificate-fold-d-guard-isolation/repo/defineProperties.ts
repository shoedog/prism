declare const module: { exports: object };
export function f(/*@defineProperties-formal*/formal: number) { return formal; }
Object.defineProperties(module.exports, { marker: { value: 1 } });
