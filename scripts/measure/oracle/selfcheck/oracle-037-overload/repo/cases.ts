function f(a: number): number;
function f(b: string): string;
function f(x: number | string) { return x; }
function run() { return f(42); }
