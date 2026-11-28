export {};
function id<T>(x: T) { return x; }
const f1 = (id)<number>;
const f2 = id<number>;
const f3 = id(1);
declare const cond: boolean;
declare const other: typeof id;
const f4 = ((id))<number>;
const f5 = (cond ? id : other)<number>;
void [f1, f2, f3, f4, f5];
