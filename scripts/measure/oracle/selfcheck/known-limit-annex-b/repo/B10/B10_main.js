function go() {
/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
const x = 7;
for (let j = 0; j < 1; j++) {
/** @param {number} b */
function /*@fresh*/f(/*@b*/b) { return "fresh:" + b; }
}
return /*@site*/ /*@read*/f(/*@actual*/x);
}
console.log(go());
