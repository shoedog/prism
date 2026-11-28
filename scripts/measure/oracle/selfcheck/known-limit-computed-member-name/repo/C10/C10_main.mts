const /*@out*/k = "outer";
function key(get: () => string): "m" { console.log(get()); return "m"; }
class D { [key(() => /*@read*/k)]() { const /*@local*/k = "inner"; return k; } }
console.log(typeof D);
