import { exec } from 'child_process';
function View(p: { x: string }) {
  p.x = 'fixed';
  exec(p.x); // @SINK
  return null;
}
function entry() {
  const t = acquire(); // @SRC
  return <View {...{x: t}}/>;
}
