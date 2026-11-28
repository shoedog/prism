function go() {
/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
const x = 7;
label: {
if (x === 7) break label;
/** @param {number} b */
function /*@fresh*/f(/*@b*/b) { return "fresh:" + b; }
}
return /*@site*/ /*@read*/f(/*@actual*/x);
}
console.log(go());
