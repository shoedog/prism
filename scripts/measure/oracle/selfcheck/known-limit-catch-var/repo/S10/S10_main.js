function go() {
try { throw "catch"; } catch (/*@param*/e) {
const sequence = /** @type {any} */ (["loop"]); for (var /*@var*/e of sequence) { console.log(/*@inside*/e); }
}
function readOutside() { return typeof /*@outside*/e; } console.log(readOutside());
}
go();
