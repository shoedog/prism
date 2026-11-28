class /*@oldClass*/Factory {
 constructor(/** @type {number} */ /*@a*/a) { console.log("old:" + a); }
 /*@old*/run(/** @type {number} */ /*@m*/m) {return "old-method:" + m;}
}
function go() {
{
/** @param {number} b */
function /*@newClass*/Factory(/*@b*/b) {console.log("new:" + b); return { /*@fresh*/run(/** @type {number} */ /*@n*/n) {return "new-method:" + n;} };}
}
return /*@site*/(/*@make*/new /*@read*/Factory(/*@actual*/7)).run(/*@actualMethod*/8);
}
console.log(go());
