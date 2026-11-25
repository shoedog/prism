import Koa from 'koa';
const app = new Koa();
app.use(async (ctx) => {
  const token = ctx.cookies.get('token');
  ctx.body = token;
});
