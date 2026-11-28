export {};
namespace NM { export function fm(m1: number) { return m1; } }
// @ts-ignore
var NM = { fm: function (mr: number) { return mr; } };
export function runm(xm: number) { return NM.fm(xm); }
