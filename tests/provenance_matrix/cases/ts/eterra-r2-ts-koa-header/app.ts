import Koa from "koa";
const app = new Koa();
app.use(async (ctx) => {
  const token = ctx.request.headers.authorization;
  console.log(token);
});
