export {};
declare function use(...v: unknown[]): void;
class Holder { constructor(public app: number) {} }
namespace NSP { export const app2 = 2; }
export function p1(req: Holder) {
  const app = 1;
  use(req.app, app);
  return app;
}
export function p2() {
  const app2 = 1;
  use(NSP.app2, app2);
  return app2;
}
