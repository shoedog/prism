export {};
const method = 1;
const signature = 2;
const getter = 3;
const setter = 4;
class C {
  method() {}
  get getter() { return 0; }
  set setter(n: number) {}
}
interface I { signature(): void; }
const reads = [method, signature, getter, setter];
