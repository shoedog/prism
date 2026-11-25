import { exec } from 'child_process';
function View(p: { obj: { x: string } }) {
  observe(p.obj); // @SINK
  return null;
}
function entry() {
  const t = acquire(); // @SRC
  return <><View obj={{ x: 'fixed' }}/><View obj={{
    x: t
  }}/></>;
}
