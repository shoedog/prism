/** @param {number} n */
const go = function /*@self*/f(n) {
if (n === 0) return "self";

const x = 7;
{
/** @param {number} b */
function /*@fresh*/f(/*@b*/b) { return "fresh:" + b; }
}
return /*@site*/ /*@read*/f(/*@actual*/x);
}
console.log(go(1));
