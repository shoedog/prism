class Base { /*@decl*/m(){return "class";} }
/** @type {Base} */
const separate = { /*@own*/m(){return "literal";} };
console.log(/*@site*/separate./*@read*/m());
