import Koa from "koa";

const app = new Koa();
const allowedHosts = ["example.com"];

app.use(async (ctx, next) => {
  const target = ctx.request.body.url;
  const parsed = new URL(target);
  if (!allowedHosts.includes(parsed.hostname)) {
    return;
  }
  return fetch(target);
});

