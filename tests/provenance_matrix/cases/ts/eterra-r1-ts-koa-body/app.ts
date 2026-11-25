import Koa from 'koa';
const app = new Koa();
app.use(async (ctx) => {
  const payload = ctx.request.body;
  console.log(payload);
});
