class /*@oldClass*/Factory {
 constructor(/** @type {number} */ /*@a*/a) { console.log("old:" + a); }
 static /*@oldStatic*/direct(/** @type {number} */ /*@s*/s) {return "old-static:" + s;}
 /*@old*/run(/** @type {number} */ /*@m*/m) {return "old-method:" + m;}
}
function go() {
class Replacement { constructor(/** @type {number} */ /*@b*/b) { console.log("new:" + b); }
 static /*@newStatic*/direct(/** @type {number} */ /*@t*/t) {return "new-static:" + t;}
 /*@fresh*/run(/** @type {number} */ /*@n*/n) {return "new-method:" + n;}
}
eval("var Factory = Replacement;");
class Derived extends Factory { read() { return /*@superCall*/super.run(/*@sa*/9); } }
console.log(/*@site*/(/*@make*/new /*@read*/Factory(/*@actual*/7)).run(/*@actualMethod*/8));
console.log(/*@staticCall*/Factory.direct(/*@sta*/10));
console.log(new Derived(11).read());
}
go();
