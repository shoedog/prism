import express from 'express';

const router = express.Router();

function list(req: express.Request, res: express.Response): void {
  res.json([]);
}

router.get('/users', list);

export default router;
