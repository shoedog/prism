class A { /*@adecl*/n() { return "A"; } run() { return this./*@read*/n(); } }
class B extends A { /*@bdecl*/n() { return "B"; } }
console.log(new B().run());
