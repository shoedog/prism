/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
function go(snapshot = /*@defaultSite*/ /*@defaultRead*/f(/*@defaultActual*/9)) {
const x = 7;
{
/** @param {number} b */
function /*@fresh*/f(/*@b*/b) { return "fresh:" + b; }
}
return snapshot + "|" + /*@site*/ /*@read*/f(/*@actual*/x);
}
console.log(go());
