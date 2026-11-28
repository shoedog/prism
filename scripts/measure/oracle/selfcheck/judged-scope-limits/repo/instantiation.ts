export {};
function id<T>(x: T) { return x; }
const f1 = (id)<number>;
const f2 = id<number>;
const f3 = id(1);
void [f1, f2, f3];
