declare const module: { exports: object };
export function f(/*@setPrototypeOf-formal*/formal: number) { return formal; }
Object.setPrototypeOf(module.exports, {});
