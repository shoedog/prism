import express from 'express';
const app = express();
app.get('/catalog/:id', (incoming, reply) => {
  const address = incoming.originalUrl;
  const pattern = incoming.route.path;
  reply.json([address, pattern]);
});
