import { exec } from 'child_process';
declare function acquire(): string;
function last<T extends string>(x: T) {
  exec(x); // @SINK
}
function middle<T extends string>(x: T) {
  last<T>(x);
}
function first<T extends string>(x: T) {
  middle<T>(x);
}
function main() {
  const t = acquire(); // @SRC
  first<string>(t);
}
main();
