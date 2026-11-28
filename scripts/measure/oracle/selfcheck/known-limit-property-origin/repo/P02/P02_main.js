const original = { /*@def*/q: 1 };
/** @type {typeof original} */
const other = { /*@own*/q: 2 };
const { /*@read*/q: value } = other;
console.log(original.q, value);
