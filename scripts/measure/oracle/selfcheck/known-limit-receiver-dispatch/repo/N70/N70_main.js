class Base {}
const M1 = (/** @type {typeof Base} */ B) => class extends B { /*@m1*/m() { return "M1"; } };
const M2 = (/** @type {typeof Base} */ B) => class extends B { /*@m2*/m() { return "M2"; } };
class C extends M2(M1(Base)) {}
console.log(new C()./*@read*/m());
