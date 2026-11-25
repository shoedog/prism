import express from 'express';
const app = express();
class Preview {
  static local(req: { query: { theme: string } }) {
    const theme = req.query.theme;
    return theme;
  }
  static live(req: express.Request, res: express.Response) {
    const theme = req.query.theme;
    res.json(theme);
  }
}
app.get('/preview', Preview.live);
Preview.local({ query: { theme: 'sample' } });
