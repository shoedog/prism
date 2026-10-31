import { readConfig } from './read';
import { readConfig2 } from './read2';
import { Request, Response } from 'express';
export function route(req: Request, res: Response) {
  const out = readConfig({ name: req.query.name as string });
  const out2 = readConfig(req.query as any);
  const out3 = readConfig2({ name: req.query.name as string });
  const out4 = readConfig2(req.query as any);
  res.send(out + out2 + out3 + out4);
}
