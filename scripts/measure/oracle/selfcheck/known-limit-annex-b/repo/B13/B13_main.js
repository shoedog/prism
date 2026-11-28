function go() {
/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
const x = 7;
function readLater() { return /*@site*/ /*@read*/f(/*@actual*/x); }
{
/** @param {number} b */
function /*@fresh*/f(/*@b*/b) { return "fresh:" + b; }
}
return readLater();
}
console.log(go());
