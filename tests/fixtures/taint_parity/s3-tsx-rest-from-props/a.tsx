import { exec } from 'child_process';
function View({ skip, ...rest }: { skip: string, x: string }) {
  exec(rest.x); // @SINK
  return null;
}
function entry() {
  const t = acquire(); // @SRC
  return <View skip="fixed" x={t}/>;
}
