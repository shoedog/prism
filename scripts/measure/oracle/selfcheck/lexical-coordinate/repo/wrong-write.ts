export {};
declare const browserURL: { createObjectURL(f: unknown): string };
declare function use(v: unknown): void;
export function blob(file: unknown) {
  let objectURL: string | null = null;
  try {
    (objectURL = browserURL.createObjectURL(file));
  } catch (e) {
    objectURL = null;
  }
  use(objectURL);
}
export function quad(b: number, c: number) {
  let t2 = Infinity;
  let t1 = 0;
  t1 = t2 = -c / b;
  use(t2);
  return t1;
}
