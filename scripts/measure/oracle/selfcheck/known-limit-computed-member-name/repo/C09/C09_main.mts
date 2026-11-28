const /*@out*/k = "outer";
function key(get: () => string): "m" { console.log(get()); return "m"; }
const o = { [key(() => /*@read*/k)](/*@param*/k = "inner") { return k; } };
console.log(o.m());
