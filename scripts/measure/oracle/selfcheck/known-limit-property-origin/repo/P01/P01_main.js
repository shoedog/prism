const original = { /*@def*/q: 1 };
/** @type {typeof original} */
const other = { /*@own*/q: 2 };
console.log(original.q, other./*@read*/q);
