const original = { /*@def*/q: 1 };
/** @type {typeof original} */
const other = original;
console.log(original.q, other./*@read*/q);
