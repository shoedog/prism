import { exec } from 'child_process';
class Service {
  async run(o: { x: string, stable: string }) {
    exec(o.x); // @SINK
    return o.stable;
  }
}
function getService() { return new Service(); }
async function entry() {
  const t = acquire(); // @SRC
  await getService().run({ x: t!, stable: 'fixed' });
}
