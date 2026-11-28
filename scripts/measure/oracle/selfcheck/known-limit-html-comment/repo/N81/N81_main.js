/** @param {number} a */
function /*@old*/f(/*@a*/a) { return "old:" + a; }
/** @type {any} */
const q = f(7);
let /*@decl*/w = 1;
const r = q <!--/*@read*/w
;
console.log(r, w);
