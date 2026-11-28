const key: string = "f";
const o = {
 /*@old*/f(/*@a*/a = 1) { return "first:" + a; },
 [key](/*@b*/b = 1) { return "computed:" + b; }
};
console.log(/*@site*/o./*@read*/f(/*@actual*/7));
