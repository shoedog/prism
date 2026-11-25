import express from 'express';

const router = express.Router();
router.param('item', (_req, _res, next, rawKey) => {
  const itemKey = rawKey;
  console.log(itemKey);
  next();
});
router.get('/items/:item', (_req, res) => res.sendStatus(204));
