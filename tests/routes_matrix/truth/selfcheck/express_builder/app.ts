import express from 'express';
const app = express();
app.get('env');
app.route('/x').get((_req: unknown, res: any) => res.end()).post((_req: unknown, res: any) => res.send('ok'));
app.all('/any', function any(_req, res) { res.end(); });
app.route('/head').get(function show(_req, res) { res.end(); }).head(function head(_req, res) { res.end(); });
export default app;
