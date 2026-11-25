import { exec } from 'child_process';
function validate(x: string) {
  return { error: false, value: { range: x } };
}
function entry() {
  const t = acquire(); // @SRC
  const { error, value } = validate(
    t
  );
  if (error) return;
  const x = value.range;
  exec(x); // @SINK
}
