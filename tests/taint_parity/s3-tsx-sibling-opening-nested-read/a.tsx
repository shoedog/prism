import { exec } from 'child_process';
function View(p: { x: string }) {
  function later() {
    exec(p.x); // @SINK
  }
  later();
  return null;
}
function entry() {
  const t = acquire(); // @SRC
  return <><View x={t}/><View x="fixed"/></>;
}
