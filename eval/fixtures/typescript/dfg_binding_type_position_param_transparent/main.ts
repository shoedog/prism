function localName(): void {
  let x = source();
  let g: ({ x }: Box) => void = h;
  sink(x);
}

function parameterName(x: number): void {
  let g: ({ x }: Box) => void = h;
  sink(x);
}
