const /*@original*/holder = { /*@old*/m(/** @type {number} */ /*@a*/a){return "old:" + a;} };
function go() {
 const replacement = { /*@fresh*/m(/** @type {number} */ /*@b*/b){return "fresh:" + b;} };
 eval("var holder = replacement;");
 return /*@site*/ /*@read*/holder.m(/*@actual*/7);
}
console.log(go());
