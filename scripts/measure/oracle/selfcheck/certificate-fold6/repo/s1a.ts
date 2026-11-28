namespace SA { export function f(sa1: number) { return sa1; } }
// @ts-ignore
globalThis.SA = { f: function (ra1: number) { return ra1; } };
function runsa(xa: number) { return SA.f(xa); }
