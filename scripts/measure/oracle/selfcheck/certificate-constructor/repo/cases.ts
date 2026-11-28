class A { m = (a1: number) => a1; }
class B { m = (b1: number) => b1; }
declare function pick(): typeof A;
function viaParam(K: typeof A, v1: number) { return new K().m(v1); }
function viaCall(v2: number) { return new (pick())().m(v2); }
let L = class { m = (l1: number) => l1; };
L = class { m = (l2: number) => l2; };
function viaLet(v3: number) { return new L().m(v3); }
class R { m = (r1: number) => r1; constructor() { return new B() as any; } }
function viaReturn(v4: number) { return new R().m(v4); }
class S extends R { }
function viaBase(v5: number) { return new S().m(v5); }
class OK { m = (o1: number) => o1; constructor() { return; } }
function viaOk(v6: number) { return new OK().m(v6); }
function direct(v7: number) { return new A().m(v7); }
