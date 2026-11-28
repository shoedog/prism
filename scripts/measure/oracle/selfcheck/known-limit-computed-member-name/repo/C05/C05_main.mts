function /*@old*/tag(/*@a*/a = 1): "m" { console.log("outer:" + a); return "m"; }
function key(get: () => "m"): "m" { return get(); }
class D {
 [key(() => /*@site*/ /*@read*/tag(/*@actual*/7))]() {
function /*@new*/tag(/*@b*/b = 1): "m" { console.log("inner:" + b); return "m"; }
 return tag(9); }
}
console.log(typeof D);
