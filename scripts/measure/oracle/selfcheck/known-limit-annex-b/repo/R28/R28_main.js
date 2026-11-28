/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
function go() {
try { throw "catch-value"; } catch (/*@param*/f) {
{
/** @param {number} b */
function /*@fresh*/f(/*@b*/b) { return "fresh:" + b; }
}
console.log(/*@inner*/f);
}
return /*@site*/ /*@read*/f(/*@actual*/7);
}
console.log(go());
