import Koa from 'koa';
const app = new Koa();
app.use(async (ctx) => {
  const agent = ctx.request.header['user-agent'];
  const method = ctx.method;
  return [agent, method];
});
