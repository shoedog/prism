import express from 'express';
import type { Request, Response, NextFunction } from 'express';
const app = express();
app.use((failure: Error, incoming: Request, reply: Response, next: NextFunction) => {
  const label = incoming.query.label;
  const details = [label, failure.message];
  reply.status(400).json(details);
});
