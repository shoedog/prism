class A { /*@adecl*/x = 1; get() { return this./*@read*/x; } }
class B extends A { declare /*@bdecl*/x: number; }
console.log(new B().get());
