function go() {
try { throw "catch"; } catch (/*@param*/e) {
for (var /*@var*/e in { loop: 1 }) { console.log(/*@inside*/e); }
}
function readOutside() { return typeof /*@outside*/e; } console.log(readOutside());
}
go();
