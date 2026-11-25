import Koa from 'koa';
import { readdir as list } from 'node:fs';
const app = new Koa();
function isolated() {
  const t = acquire(); // @SRC
  observe(t);
}
app.use(ctx => {
  const value = ctx.query.value;
  list(value); // @SINK
});
