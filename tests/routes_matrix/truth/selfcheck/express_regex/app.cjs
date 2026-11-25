const express = require('express');
const router = express.Router();
router.get('/x', (_req, res) => res.end());
const app = express();
app.use(/^\/v[12]/, router);
module.exports = app;
