class A { /*@adecl*/x = 1; }
class B extends A { declare /*@bdecl*/x: number; }
const b = new B();
console.log(b./*@read*/x);
