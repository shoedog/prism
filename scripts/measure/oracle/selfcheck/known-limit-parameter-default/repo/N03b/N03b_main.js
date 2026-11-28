function go() { return 1; }
/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
const run = (/** @type {() => string} */ g = () => /*@site*/ /*@read*/f(/*@actual*/7)) => {
/** @param {number} b */
function /*@fresh*/f(/*@b*/b) { return "fresh:" + b; }
return g();
};
console.log(run());
