import { Request, Response } from 'express';
function inspect(req: Request, res: Response) {
  const value = req.query.q;
  res.send(value);
}
