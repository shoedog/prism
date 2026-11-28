namespace SB { export function f(sb1: number) { return sb1; } }
// @ts-ignore
var SB = { f: function (rb1: number) { return rb1; } };
function runsb(xb: number) { return SB.f(xb); }
