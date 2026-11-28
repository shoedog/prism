/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
const /*@alias*/g = f;
function go() {
const x = 7;
/** @param {number} b */
function /*@fresh*/implementation(/*@b*/b) { return "fresh:" + b; }
eval("var g = implementation;");
return /*@site*/ /*@read*/g(/*@actual*/x);
}
console.log(go());
