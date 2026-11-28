class A { /*@adecl*/x = 1; }
class B extends A {}
interface B { /*@bdecl*/x: number; }
const b = new B();
console.log(b./*@read*/x);
