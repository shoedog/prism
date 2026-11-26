function /*@old*/f() { return "old"; }
function go(/** @type {() => string} */ g = () => /*@read*/f()) {
 function /*@fresh*/f() { return "fresh"; }
 return g() + "|" + /*@bodyread*/f();
}
console.log(go());
