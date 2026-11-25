import { exec } from 'child_process';
class Message {
  constructor(p: { reason: string }) {
    exec(p.reason); // @SINK
  }
}
function entry() {
  const t = acquire(); // @SRC
  if (permitted(t)) return;
  throw new Message({ reason: `${t} is unavailable` });
}
