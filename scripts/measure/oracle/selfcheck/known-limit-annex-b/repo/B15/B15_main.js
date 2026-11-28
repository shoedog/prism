function go() {
/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
const x = 7;
{
/** @param {number} b */
function /*@fresh*/f(/*@b*/b) { return "fresh:" + b; }
}
const alias = /*@read*/f;
return /*@site*/alias(/*@actual*/x);
}
console.log(go());
