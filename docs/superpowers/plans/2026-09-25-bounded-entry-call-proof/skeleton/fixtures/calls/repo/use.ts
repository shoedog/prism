import { both, mid, fieldOnly, bare } from "./lib";
export function run(n: number, later: number, st: { x: number }) {
  const p = both(n, later);
  const q = mid(n, { pad: 1 }, later);
  const r = fieldOnly({ pad: 2 }, st);
  const s = bare({ pad: 3 }, st);
  const t = mid(n, { pad: 1 }, true);
  const u = bare({ pad: 4 }, st);
  const v = both(/* c */ n, later);
  return p + q + r + s + t + u + v;
}
