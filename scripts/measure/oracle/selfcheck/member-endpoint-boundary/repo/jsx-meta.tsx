declare function C(p: { foo: number; 'ns:bar'?: number }): any;
const foo = 1;
<C
  foo={2}
  ns:bar={2}
/>;
const readFoo = foo;
const bar = 2;
const readBar = bar;
const target = 3;
function ctor() { return new.target; }
const readTarget = target;
export {};
