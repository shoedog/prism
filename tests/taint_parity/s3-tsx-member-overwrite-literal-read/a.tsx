import { exec } from 'child_process';
function View(p: { bad: string, safe: string }) {
  exec(p['safe']); // @SINK
  return null;
}
const UI = { View };
function entry() {
  const t = acquire(); // @SRC
  return <UI.View safe={t} {...{safe: 'fixed'}} bad={t}/>;
}
