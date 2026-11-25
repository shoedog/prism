import Koa from "koa";
const app = new Koa();
app.use(async (ctx) => {
  const method = ctx.request.method;
  return method;
});
