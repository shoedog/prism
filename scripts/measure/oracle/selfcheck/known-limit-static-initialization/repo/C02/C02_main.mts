class B { static readonly /*@old*/run = (/*@a*/a: number) => "base:" + a; }
class D extends B {
static probe() { return /*@site*/D./*@read*/run(/*@actual*/7); }
static readonly /*@new*/run = (/*@b*/b: number) => "derived:" + b;
static { console.log(D.probe()); }
}
