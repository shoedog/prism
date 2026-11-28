"use\x20strict";
function go() {
/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
{
/** @param {number} b */
function /*@fresh*/f(/*@b*/b) { return "fresh:" + b; }
}
return /*@site*/ /*@read*/f(/*@actual*/7);
} console.log(go());
