import { exec } from 'child_process';
declare function acquire(): string;
function Box(props: {cmd: string}) {
  exec(props.cmd); // @SINK
  return null;
}
function main() {
  const t = acquire(); // @SRC
  const fixed: Partial<{cmd: string}> = {cmd: 'fixed'};
  return <Box {...fixed} cmd={t} />;
}
