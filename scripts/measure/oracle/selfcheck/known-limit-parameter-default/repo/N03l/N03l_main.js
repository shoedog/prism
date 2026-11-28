function outer(/** @type {() => string} */ /*@old*/f) {
 function go(/** @type {() => string} */ g = () => /*@read*/f()) {
 function /*@fresh*/f() { return "fresh"; }
 return g();
 }
 return go();
}
console.log(outer(() => "old"));
