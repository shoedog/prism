export {};
namespace N {
  export function h(v4: number) { return v4; }
  export function k(w4: number) { return w4; }
  // @ts-ignore
  h = k;
}
export function run(x4: number) { return N.h(x4); }
