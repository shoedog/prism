import express from 'express';
const app = express();
app.use(express.json());
app.post('/preview', (incoming, reply) => {
  incoming.body.title = 'preview';
  const title = incoming.body.title;
  const caption = incoming.body.caption;
  reply.json([title, caption]);
});
