declare namespace JSX { interface IntrinsicElements { div: {}; } }
const div = 1;
const Comp = (): any => null;
const intrinsic = (
  <div />
);
const component = (
  <Comp />
);
function f(p: number) { return p; }
const v = 1;
f(v);
