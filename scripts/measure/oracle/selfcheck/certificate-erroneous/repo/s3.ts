export {};
class B3 { readonly r = (v3: number) => v3; }
class C3 extends B3 { g(x3: number) {
  // @ts-ignore
  return super.r(x3); } }
