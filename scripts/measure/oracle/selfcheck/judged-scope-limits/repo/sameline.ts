export {};
const obj = { x: 1 };
let x = 0;
x = 1; void obj.x;
let y = 0;
y = 1;
const fa = function nm() { return 1; }, fb = function nm() { return nm; };
const fc = function nm() { return 1; };
const fd = function nm() { return nm; };
let z = 0;
z = 1; z: {}
const ka = class Cn { m() { return 1; } }, kb = class Cn { m() { return Cn; } };
const kc = class Cn { m() { return 1; } };
const kd = class Cn { m() { return Cn; } };
{ enum Qe { A } } { enum Qe { B } void Qe; }
{ enum Qe { A } }
{ enum Qe { B } void Qe; }
void [x, y, z, fa, fb, fc, fd, ka, kb, kc, kd];
