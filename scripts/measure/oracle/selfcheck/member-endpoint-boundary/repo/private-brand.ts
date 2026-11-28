class C {
  #x = 1;
  m(o: object) {
    return #x in o;
  }
  n(o: object) {
    return #x in o;
  }
}
class D {
  #m = 1;
  test(o: object) {
    const m = o;
    const a = #m in m;
    return a && #m in o;
  }
}
const x = new C();
const read = x;
export {};
