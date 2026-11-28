/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
function go() {
{
if (true)
/** @param {number} b */
function /*@fresh*/f(/*@b*/b = 0) { return "fresh:" + b; }
}
return /*@site*/ /*@read*/f(/*@actual*/7);
} console.log(go());
