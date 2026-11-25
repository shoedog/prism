import express, { type Request, type Response } from 'express';

const router = express.Router();
function show(incoming: Request, reply: Response) {
  const itemKey = incoming.params.item;
  reply.send(itemKey);
}
router.route('/items/:item').get(show);
