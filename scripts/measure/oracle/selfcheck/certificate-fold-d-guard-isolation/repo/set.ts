declare const module: { exports: object };
export function f(/*@set-formal*/formal: number) { return formal; }
Reflect.set(module.exports, 'marker', 1);
