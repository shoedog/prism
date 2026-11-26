function go() {
/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
const x = 7;
switch (x) {
case 7:
/** @param {number} b */
function /*@fresh*/f(/*@b*/b) { return "fresh:" + b; }
break;
}
return /*@site*/ /*@read*/f(/*@actual*/x);
}
console.log(go());
