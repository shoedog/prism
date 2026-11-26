/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
function go() {
const x = 7;
return typeof /*@read*/f;
{
/** @param {number} b */
function /*@fresh*/f(/*@b*/b) { return "fresh:" + b; }
}
}
console.log(go());
