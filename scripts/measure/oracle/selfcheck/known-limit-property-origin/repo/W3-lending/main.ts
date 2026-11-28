const o = { /*@lender*/q: 1 };
const o2: typeof o = { /*@own*/q: 2 };
console.log(o.q, o2./*@read*/q);
