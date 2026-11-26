class A { static /*@adecl*/n() { return "A"; } static run() { return this./*@read*/n(); } }
class B extends A { static /*@bdecl*/n() { return "B"; } }
console.log(B.run());
