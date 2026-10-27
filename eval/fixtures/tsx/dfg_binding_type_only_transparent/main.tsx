function f(): void {
  let x = source();
  type T = { x: number };
  interface I { x: string }
  enum E { x = 1 }
  declare namespace N { const x: number; }
  function generic<x>(): void {}
  let functionType: ({ x }: Box) => void = h;
  let constructorType: new ({ x }: Box) => Thing = h;
  let signatures: { ({ x }: Box): void; new ({ x }: Box): Thing; m({ x }: Box): void } = h;
  function declared({ x }: Box): void;
  function declared(a: any): void {}
  abstract class A { abstract m({ x }: Box): void; }
  sink(x);
}
