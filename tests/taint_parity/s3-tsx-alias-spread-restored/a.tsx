import { exec } from 'child_process';
function View(p: { x: string }) {
  exec(p.x); // @SINK
  return null;
}
const Alias = View;
function entry() {
  const t = acquire(); // @SRC
  return <><View x="fixed"/><Alias {...{x: 'fixed'}} x={t}/></>;
}
