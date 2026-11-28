class A { /*@old*/m = (a: number) => "A:" + a; }
class B extends A { declare /*@fresh*/m: (b: number) => string; }
const b = new B();
console.log(/*@site*/b./*@read*/m(/*@actual*/7));
