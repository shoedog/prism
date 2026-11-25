import express, { type Request, type Response } from 'express';
const router = express.Router();
router.use('/search', function read(incoming: Request, outgoing: Response) {
  const phrase = incoming.query.q;
  outgoing.send(phrase);
});
const app = express();
app.use(router);
