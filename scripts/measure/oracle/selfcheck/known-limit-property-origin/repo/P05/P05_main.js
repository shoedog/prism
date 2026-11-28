const base = { __proto__: null, /*@own*/q: 42 };
const object = { /*@def*/__proto__: base };
console.log(typeof object./*@read*/__proto__, "__proto__" in object);
