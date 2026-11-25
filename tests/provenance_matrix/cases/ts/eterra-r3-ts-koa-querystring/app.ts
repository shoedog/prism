import Koa from 'koa';
const app = new Koa();
app.use(async (ctx) => {
  const term = ctx.query.q;
  ctx.body = term;
});
