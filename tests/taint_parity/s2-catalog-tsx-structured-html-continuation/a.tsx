import { exec } from 'child_process';
declare function acquire(): string;
function entry() {
  const t = acquire(); // @SRC
  return <section
    dangerouslySetInnerHTML={{
      __html: String(t), // @SINK
    }}
  />;
}
entry();
