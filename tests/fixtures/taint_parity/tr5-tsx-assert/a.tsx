import { exec } from 'child_process';
function run(x: string) {
  exec(x); // @SINK
}
function main() {
  const t: string | null = acquire(); // @SRC
  run((t! as string));
}
main();
