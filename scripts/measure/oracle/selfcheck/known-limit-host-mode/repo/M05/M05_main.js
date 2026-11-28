function /*@fun*/sample() {return "global-fn";}
var /*@var*/visible = "global-var";
console.log(globalThis./*@readFn*/sample?.(), globalThis./*@readVar*/visible);
