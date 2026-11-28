export class A {
  m = 0;
  sameMethod(write: boolean) {
    if (write) this./*@thisWrite*/m = 1;
    return this./*@thisRead*/m;
  }
}
export function crossInstance() {
  new A().sameMethod(true);
  return new A().sameMethod(false);
}
export function varRedeclaration() {
  var a = new A();
  a./*@varWrite*/m = 1;
  var a = new A();
  return a./*@varRead*/m;
}
export function letReassignment() {
  let a = new A();
  a./*@letWrite*/m = 1;
  a = new A();
  return a./*@letRead*/m;
}
export function sameInstance() {
  const a = new A();
  a./*@sameWrite*/m = 1;
  return a./*@sameRead*/m;
}
export function perIteration() {
  let result = 0;
  for (let i = 0; i < 2; i++) {
    const a = new A();
    result = a./*@loopRead*/m;
    a./*@loopWrite*/m = 1;
  }
  return result;
}
export function localPlain(write: boolean) {
  let cell = 0;
  if (write) /*@localWrite*/cell = 1;
  return /*@localRead*/cell;
}
export function varPlain() {
  var cell = 0;
  /*@varPlainWrite*/cell = 1;
  cell = 0;
  return /*@varPlainRead*/cell;
}
export function letPlain() {
  let cell = 0;
  /*@letPlainWrite*/cell = 1;
  cell = 0;
  return /*@letPlainRead*/cell;
}
export function controlPlain() {
  let /*@samePlainWrite*/cell = 1;
  return /*@samePlainRead*/cell;
}
export function loopPlain() {
  let result = 0;
  for (let i = 0; i < 2; i++) {
    let cell = 0;
    result = /*@loopPlainRead*/cell;
    /*@loopPlainWrite*/cell = 1;
  }
  return result;
}
const obj = { /*@origin*/q: 1 };
export const obj2: typeof obj = { q: 2 };
export const borrowed = obj2./*@borrowed*/q;
const /*@outerPlain*/q = 1;
export function shadowed() {
  const /*@innerPlain*/q = 2;
  return /*@shadowRead*/q;
}
