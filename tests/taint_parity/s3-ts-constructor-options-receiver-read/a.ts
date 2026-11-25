import { exec } from 'child_process';
class Service {
  constructor(private props: { x: string }) {}
  run() {
    exec(this.props.x); // @SINK
  }
}
function entry() {
  const t = acquire(); // @SRC
  const service = new Service({ x: t });
  service.run();
}
