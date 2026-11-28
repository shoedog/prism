declare const module: { exports: object };
export function f(/*@deleteProperty-formal*/formal: number) { return formal; }
Reflect.deleteProperty(module.exports, 'marker');
