function gf(old1) { return old1; }
const gt = globalThis;
gt.gf = function (rep1) { return rep1; };
function grun(x4) { return gf(x4); }
