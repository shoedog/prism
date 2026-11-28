declare const module: { exports: object };
export function f(/*@assign-formal*/formal: number) { return formal; }
Object.assign(module.exports, { marker: 1 });
