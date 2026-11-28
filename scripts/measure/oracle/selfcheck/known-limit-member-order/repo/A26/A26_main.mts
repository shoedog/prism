const extras: {} = { f(b = 1) { return "spread:" + b; } };
const o = { /*@old*/f(/*@a*/a = 1) { return "first:" + a; }, ...extras };
console.log(/*@site*/o./*@read*/f(/*@actual*/7));
