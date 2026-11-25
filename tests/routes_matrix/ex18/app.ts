import express from 'express';

const app = express();

function health(req: express.Request, res: express.Response): void {
  res.sendStatus(204);
}

app.get('/health', health);

export default app;
