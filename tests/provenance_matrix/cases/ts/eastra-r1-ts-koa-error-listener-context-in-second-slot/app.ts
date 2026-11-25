import Koa from 'koa';
const app = new Koa();
app.on('error', (failure: Error, context?: Koa.Context) => {
  if (!context) return;
  const dialect = context.get('Accept-Language');
  void [failure, dialect];
});
app.use(() => { throw new Error('unavailable'); });
