let f = ((v: number) => v);
f = (w: number) => w;
const g = ((x: number) => x);
function run(a: number, b: number) { return [f(a), g(b)]; }
