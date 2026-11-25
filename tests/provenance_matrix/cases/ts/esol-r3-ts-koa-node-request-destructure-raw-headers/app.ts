import Koa from 'koa';
const app = new Koa();
app.use(context => {
  const { req: incoming } = context;
  const raw = incoming.rawHeaders;
  const outgoing = context.res.headersSent;
  context.body = [raw, outgoing];
});
