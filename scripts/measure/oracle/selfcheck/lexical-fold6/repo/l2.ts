export function l2(src: { app: number }) {
  let app: number;
  ({ app: app } = src);
  return app;
}
