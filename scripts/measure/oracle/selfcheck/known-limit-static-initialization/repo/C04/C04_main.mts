class B { static /*@old*/run(/*@a*/a: number) { return "base:" + a; } }
class D extends B {
static probe() { return /*@site*/D./*@read*/run(/*@actual*/7); }
static { console.log(D.probe()); }
static readonly /*@new*/run = (/*@b*/b: number) => "derived:" + b;
}
