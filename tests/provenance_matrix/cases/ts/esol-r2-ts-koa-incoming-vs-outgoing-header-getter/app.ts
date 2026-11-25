import Koa from 'koa';
const app = new Koa();
app.use((incoming) => {
  incoming.set('X-Label', 'fixed');
  const requested = incoming.get('X-Label');
  const sent = incoming.response.get('X-Label');
  incoming.body = [requested, sent];
});
