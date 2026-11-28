/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
function go() {
"use strict";
const x = 7;
/** @param {number} b */
function /*@fresh*/implementation(/*@b*/b) { return "fresh:" + b; }
eval("var f = implementation;");
return /*@site*/ /*@read*/f(/*@actual*/x);
}
console.log(go());
