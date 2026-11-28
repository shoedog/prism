/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
function go(/** @type {{g?: () => string}} */ { g = () => /*@site*/ /*@read*/f(/*@actual*/7) } = {}) {
/** @param {number} b */
function /*@fresh*/f(/*@b*/b) { return "fresh:" + b; }
return g ? g() : "";
}
console.log(go());
