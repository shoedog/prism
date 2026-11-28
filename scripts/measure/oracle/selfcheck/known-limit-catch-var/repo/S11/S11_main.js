function go() {
try { throw "catch"; } catch (/*@param*/e) {
var { key: /*@var*/e } = {key:"pattern"};
console.log(/*@inside*/e);
}
function readOutside() { return typeof /*@outside*/e; } console.log(readOutside());
}
go();
