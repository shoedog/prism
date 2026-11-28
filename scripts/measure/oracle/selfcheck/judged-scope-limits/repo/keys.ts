export {};
function key(k: number) { return k; }
interface KI { [key(1)]: number }
type KT = { [key(1)]: number };
declare class KA { [key(1)]: number; }
abstract class KB { abstract [key(1)](): void; }
key(1);
