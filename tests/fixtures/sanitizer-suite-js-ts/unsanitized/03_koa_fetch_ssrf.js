import Koa from "koa";

const app = new Koa();

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  return fetch(target);
});

