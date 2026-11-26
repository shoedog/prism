export {};
function go(){
try {throw "caught";} catch (/*@param*/e) {
var /*@var*/e = "initialized";
console.log(/*@inside*/e);
}
console.log(typeof /*@outside*/e);
}
go();
