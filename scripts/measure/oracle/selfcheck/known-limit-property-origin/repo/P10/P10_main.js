class Base { /*@decl*/p = "class"; }
/** @type {Base} */
const separate = { /*@own*/p:"literal" };
console.log(new Base().p, separate./*@read*/p);
