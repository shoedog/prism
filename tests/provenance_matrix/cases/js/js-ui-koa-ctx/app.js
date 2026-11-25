const Koa = require("koa");
const app = new Koa();
app.use(ctx => {
  const term = ctx.query.term;
  return term;
});
