const express = require('express');
const app = express();
const routes = express.Router();
const partnerPrefix = '/partner';
function status(req, res) { res.send('up'); }
routes.get('/status', status);
app.use('/internal', routes);
app.use(partnerPrefix, routes);
module.exports = app;
