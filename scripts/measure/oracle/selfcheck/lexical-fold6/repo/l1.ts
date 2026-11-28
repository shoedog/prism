export {};
declare function use(...v: unknown[]): void;
export function l1(req: { app: number }) {
  const app = 1;
  use(req.app, app);
  const r = req.app ?? app;
  return r;
}
