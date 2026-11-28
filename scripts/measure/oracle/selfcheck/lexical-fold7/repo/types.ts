export {};
declare function use(...v: unknown[]): void;
const key = 'm';
type Shape = { [key]: number };
const lit = { [key]: 1 };
namespace ns { export class C {} export interface T {} }
class Impl implements ns.T {}
class Sub extends ns.C {}
interface Ext extends ns.T {}
use(lit, Impl, Sub);
export type { Shape, Ext };
