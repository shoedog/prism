function go() {
/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
const x = 7;
{
/** @param {number} b */
function /*@fresh*/f(/*@b*/b) { return "fresh:" + b; }
}
{
/** @param {number} c */
function /*@third*/f(/*@c*/c) { return "third:" + c; }
}
return /*@site*/ /*@read*/f(/*@actual*/x);
}
console.log(go());
