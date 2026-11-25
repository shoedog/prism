import { exec } from 'child_process';
declare function acquire(): string;
function run(p: {cmd: string; safe: string}) {
  const k = 'safe';
  exec(p[k]); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run({cmd: t, safe: 'fixed'});
}
main();
