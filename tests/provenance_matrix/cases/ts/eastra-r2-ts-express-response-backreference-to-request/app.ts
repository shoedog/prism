import express from 'express';
const app = express();
app.get('/books', (_incoming, outgoing) => {
  outgoing.set('X-Edition', 'preview');
  const original = outgoing.req.get('X-Edition');
  const preview = outgoing.get('X-Edition');
  outgoing.json([original, preview]);
});
