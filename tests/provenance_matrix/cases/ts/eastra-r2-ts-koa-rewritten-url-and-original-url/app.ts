import Koa from 'koa';
const app = new Koa();
app.use(ctx => {
  ctx.request.url = '/preview';
  const current = ctx.url;
  const inbound = ctx.originalUrl;
  ctx.body = [current, inbound];
});
