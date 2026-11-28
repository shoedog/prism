interface F { (v: number): number }
const f = null as unknown as F;
function run(a: number) { return f(a); }
