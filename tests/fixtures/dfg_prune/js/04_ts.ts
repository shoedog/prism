function t_decl(c: number): number {
  let x: number = c;
  x = 1;
  let y: number = x as number;
  y = x!;
  const z = y satisfies number;
  return z + y;
}

function t_ann(c: string): string {
  let s: string = c;
  s = "a";
  return s;
}
