import type { Clause as Clause } from "./types";
import { Value as Value } from "./types";
import * as ns from "./types";
function f(p: number, this: void) { return p; }
function g(this: void, q: number) { return q; }
const v = 1;
f(v, 1);
f(v);
f(1 as number);
class Impl implements ns.T {}
interface Ext extends ns.T {}
const key = "k";
type Lit = { [key]: number };
declare class Amb { [key]: number; }
const used = [Value, ns, key];
let fallback = 7;
for (const [x = fallback] of [[undefined]]) {}
void fallback;
class Box { constructor(public width: number) { void width; } }
new Box(v);
namespace Local { export const k = 1; }
import Alias = Local;
void Alias;
export {};
