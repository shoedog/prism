const o = { /*@decl*/x: 1, m() { return this./*@read*/x; } };
const m = o.m;
console.log(m());
