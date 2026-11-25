import Koa from 'koa';
const app = new Koa();
app.use(async (incoming, next) => {
  const first = incoming.get('x-edition');
  await next();
  const second = incoming.get('x-edition');
  incoming.body = [first, second];
});
app.use(async incoming => {
  incoming.req.headers['x-edition'] = 'local';
});
