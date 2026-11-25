import { exec } from 'child_process';
function View(p: { x: string, y: string }) {
  const k: 'x' = 'x';
  exec(p[k]); // @SINK
  return null;
}
function entry() {
  const t = acquire(); // @SRC
  return <><View x="fixed" y="fixed"/><View x={t} y="fixed"/></>;
}
