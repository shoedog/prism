import Koa, { type Context } from 'koa';

const app = new Koa();
app.use((incoming: Context) => {
  const { request: { query: { tag } } } = incoming;
  incoming.body = tag;
});
