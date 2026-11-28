export {};
class GS { static ['m'] = 1; }
class GD extends GS { m(w2: number) { return w2; } }
export function g1c(x15: number) { return new GD().m(x15); }
