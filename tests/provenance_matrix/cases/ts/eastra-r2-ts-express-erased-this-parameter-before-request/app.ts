import express from 'express';
const app = express();
app.get('/books/:slug', function (
  this: void,
  incoming: express.Request,
  outgoing: express.Response,
) {
  const slug = incoming.params.slug;
  outgoing.send(slug);
});
