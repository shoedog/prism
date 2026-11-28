export {};
namespace N { export const enum E { A } }
const x = N.E.A;
const x2 = N["E"]["A"];
namespace M { export const k = 1; }
const y = M.k;
const v = 1;
enum R { A = v }
const enum C { A = v }
const z = v;
void [x, y, z];
