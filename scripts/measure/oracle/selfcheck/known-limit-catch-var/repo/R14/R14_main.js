function go() {
try { throw 1; } catch (/*@param*/e) {
var /*@var*/e = 2;
console.log(/*@inside*/e);
}
console.log(typeof /*@outside*/e);
}
go();
