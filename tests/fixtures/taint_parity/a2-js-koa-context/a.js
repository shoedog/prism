import Koa from 'koa';
import { readdir as list } from 'node:fs';
const app = new Koa();
function isolated() {
  const t = acquire(); // @SRC
  observe(t);
}
function view(ctx) {
  const value = ctx.query.value;
  list(value); // @SINK
}
app.use(view);
